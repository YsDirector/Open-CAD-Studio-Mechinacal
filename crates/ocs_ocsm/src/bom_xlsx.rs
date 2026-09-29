//! 明细表 ↔ 外部 `.xlsx`（第三期）：导出 / 导入 + 「锁定数量」界面列。
//!
//! **自己读写 xlsx**（zip 容器 + SpreadsheetML），不依赖 LibreOffice/Excel/Python：
//! 只用 workspace 里已有的 `flate2`（raw deflate）、`crc32fast`（zip CRC）、
//! `quick-xml`（解析）。写出的文件是标准 xlsx，Excel / WPS / LibreOffice 都能打开。
//!
//! 列序（前 8 列 = 图纸明细表本体，第 9 列 = 只活在 xlsx 与行块 XDATA 里的界面列）：
//!
//! ```text
//! 序号 | 图号 | 名称 | 数量 | 材料 | 单重 | 总重 | 备注 | 锁定数量
//! ```
//!
//! ### 「锁定数量」列（用户 2026-09-15 定案）
//! * 空 → 不锁（数量按件数/球标引用次数算）；
//! * `Y` / `是` / `1` / `√` → 锁为**同一行「数量」列的值**；
//! * 数字 N → 锁为 N（并把「数量」列当作 N）。
//!
//! ### 导入侧"手改即锁"
//! 导出时会把当时每行的数量记进行块 XDATA（`exported`）。导入时若文件里的
//! 数量 ≠ 那个基线 → 认为**人手改过** → 自动上锁（不需要用户再勾勾）。
//!
//! ### 合并规则（导入）
//! * 文件里**非空**的单元格 → 覆盖图纸（文件是显式的编辑通道）；
//! * 文件里**空白**的单元格 → 保留图纸现值（不会把已有内容擦掉）；
//! * 文件里没有的**序号** → 保留图纸那一行（导入只改不删）；
//! * **总重**列永远忽略输入（= 单重 × 数量，算出来的）。

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use crate::bom::{RowSpec, CELL_TAGS};
use crate::i18n::{t, t_fmt};

/// xlsx 表头（9 列；第 9 列是界面列，不写进图纸明细表）。
/// ★ 中文一套 = 老文件/老图匹配键；**写入随当前语言**（见 [`headers`]），**读取两套都认**
/// （见 [`ParseHeader::is_header`] / [`parse_csv`]）。
pub(crate) const HEADERS: [&str; 9] = [
    "序号", "图号", "名称", "数量", "材料", "单重", "总重", "备注", "锁定数量",
];

/// xlsx 表头的**英文一套**（与 [`HEADERS`] 同序）。
pub(crate) const HEADERS_EN: [&str; 9] = [
    "No.",
    "Drawing No.",
    "Name",
    "Qty",
    "Material",
    "Unit wt.",
    "Total wt.",
    "Remarks",
    "Locked qty",
];

/// 当前语言的一套表头（**写入侧**：新文件随语言；旧文件中英都能导入）。
/// 前 8 列 = [`crate::bom::cell_tags`]（图纸明细表同一套列头）；第 9 列是界面列「锁定数量」。
pub(crate) fn headers(lang: crate::i18n::Lang) -> [&'static str; 9] {
    let locked = match lang {
        crate::i18n::Lang::En => HEADERS_EN[8],
        crate::i18n::Lang::Zh => HEADERS[8],
    };
    let mut h = [""; 9];
    h[..8].copy_from_slice(crate::bom::cell_tags(lang));
    h[8] = locked;
    h
}

/// 表头识别（读侧兼容）：第一格是任一语言的已知列头 ⇒ 这是表头行。
pub(crate) fn is_header_cell(cell: &str) -> bool {
    let c = cell.trim();
    HEADERS.contains(&c) || HEADERS_EN.contains(&c)
}

/// 一行 xlsx（9 格，全按文本处理；总重忽略输入）。
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct XRow {
    pub cells: [String; 9],
}

impl XRow {
    pub(crate) fn new(cells: [String; 9]) -> Self {
        XRow { cells }
    }
    pub(crate) fn item_no(&self) -> &str {
        self.cells[0].trim()
    }
    /// 8 格图纸值（不含锁定列；总重留空，由图纸重算）。
    pub(crate) fn drawing_cells(&self) -> [String; 8] {
        let mut v: [String; 8] = Default::default();
        for i in 0..8 {
            // 第 7 格（总重）不接受输入
            v[i] = if i == 6 {
                String::new()
            } else {
                self.cells[i].trim().to_string()
            };
        }
        v
    }
}

/// 一段 CSV 里某字段是否需要引号。
fn csv_need_quote(s: &str) -> bool {
    s.contains([',', '"', '\n', '\r'])
}

fn csv_field(s: &str) -> String {
    if csv_need_quote(s) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// 行 → CSV 文本（含表头；CRLF 行尾 + 无 BOM，用 UTF-8）。
pub(crate) fn csv_text(rows: &[XRow]) -> String {
    let mut out = String::new();
    out.push_str(&headers(crate::i18n::lang()).map(csv_field).join(","));
    out.push_str("\r\n");
    for r in rows {
        out.push_str(
            &r.cells
                .iter()
                .map(|c| csv_field(c))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    out
}

/// CSV → 行（支持引号包裹 / 双引号转义 / CRLF / 空行跳过 / 首行表头自动识别）。
pub(crate) fn parse_csv(text: &str) -> Result<Vec<XRow>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text); // 去 UTF-8 BOM
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut field = String::new();
    let mut line: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        field.push('"');
                    } else {
                        in_quotes = false;
                    }
                }
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => in_quotes = true,
            ',' => line.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                line.push(std::mem::take(&mut field));
                if line.iter().any(|f| !f.trim().is_empty()) {
                    rows.push(std::mem::take(&mut line));
                } else {
                    line.clear();
                }
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !line.is_empty() {
        line.push(field);
        if line.iter().any(|f| !f.trim().is_empty()) {
            rows.push(line);
        }
    }
    // 表头行：第一格是任一语言的已知列头就丢掉（旧文件中文表头 / 新文件英文表头都认）
    if rows.first().map(|r| is_header_cell(&r[0])).unwrap_or(false) {
        rows.remove(0);
    }
    let mut out = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        if r.len() > HEADERS.len() {
            return Err(t_fmt(
                "cmd.bomxlsx.err.too_many_cols",
                &[
                    ("row", &(i + 1).to_string()),
                    ("have", &r.len().to_string()),
                    ("max", &HEADERS.len().to_string()),
                ],
            ));
        }
        let mut cells: [String; 9] = Default::default();
        for (k, v) in r.iter().enumerate() {
            cells[k] = v.trim().to_string();
        }
        out.push(XRow { cells });
    }
    Ok(out)
}

// ── zip 容器 ────────────────────────────────────────────────────────────

const SIG_LOCAL: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
const SIG_CD: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
const SIG_EOCD: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];

/// 最小 zip 写手（deflate；固定 1980-01-01 时间戳 → 输出确定，便于测试）。
struct ZipWriter {
    out: Vec<u8>,
    entries: Vec<(String, u32, u32, u32, u32)>, // name, crc, comp, uncomp, offset
}

impl ZipWriter {
    fn new() -> Self {
        ZipWriter {
            out: Vec::new(),
            entries: Vec::new(),
        }
    }

    fn add(&mut self, name: &str, data: &[u8]) -> Result<(), String> {
        let mut enc = flate2::write::DeflateEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        );
        enc.write_all(data)
            .map_err(|e| t_fmt("cmd.bomxlsx.err.compress", &[("name", name), ("e", &e.to_string())]))?;
        let comp = enc
            .finish()
            .map_err(|e| t_fmt("cmd.bomxlsx.err.compress", &[("name", name), ("e", &e.to_string())]))?;
        let crc = crc32fast::hash(data);
        let off = self.out.len() as u32;
        let name_b = name.as_bytes();
        let mut h = Vec::with_capacity(30 + name_b.len());
        h.extend_from_slice(&SIG_LOCAL);
        h.extend_from_slice(&20u16.to_le_bytes()); // version needed
        h.extend_from_slice(&0u16.to_le_bytes()); // flags
        h.extend_from_slice(&8u16.to_le_bytes()); // method = deflate
        h.extend_from_slice(&0u16.to_le_bytes()); // time
        h.extend_from_slice(&0x0021u16.to_le_bytes()); // date = 1980-01-01
        h.extend_from_slice(&crc.to_le_bytes());
        h.extend_from_slice(&(comp.len() as u32).to_le_bytes());
        h.extend_from_slice(&(data.len() as u32).to_le_bytes());
        h.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
        h.extend_from_slice(&0u16.to_le_bytes()); // extra len
        h.extend_from_slice(name_b);
        self.out.extend_from_slice(&h);
        self.out.extend_from_slice(&comp);
        self.entries
            .push((name.to_string(), crc, comp.len() as u32, data.len() as u32, off));
        Ok(())
    }

    fn finish(mut self) -> Vec<u8> {
        let cd_off = self.out.len() as u32;
        for (name, crc, comp, uncomp, off) in self.entries.clone() {
            let name_b = name.as_bytes();
            let mut e = Vec::with_capacity(46 + name_b.len());
            e.extend_from_slice(&SIG_CD);
            e.extend_from_slice(&20u16.to_le_bytes()); // version made by
            e.extend_from_slice(&20u16.to_le_bytes()); // version needed
            e.extend_from_slice(&0u16.to_le_bytes()); // flags
            e.extend_from_slice(&8u16.to_le_bytes()); // method
            e.extend_from_slice(&0u16.to_le_bytes()); // time
            e.extend_from_slice(&0x0021u16.to_le_bytes()); // date
            e.extend_from_slice(&crc.to_le_bytes());
            e.extend_from_slice(&comp.to_le_bytes());
            e.extend_from_slice(&uncomp.to_le_bytes());
            e.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
            e.extend_from_slice(&0u16.to_le_bytes()); // extra
            e.extend_from_slice(&0u16.to_le_bytes()); // comment
            e.extend_from_slice(&0u16.to_le_bytes()); // disk
            e.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
            e.extend_from_slice(&0u32.to_le_bytes()); // external attrs
            e.extend_from_slice(&off.to_le_bytes());
            e.extend_from_slice(name_b);
            self.out.extend_from_slice(&e);
        }
        let n = self.entries.len() as u16;
        let cd_size = self.out.len() as u32 - cd_off;
        let mut eocd = Vec::with_capacity(22);
        eocd.extend_from_slice(&SIG_EOCD);
        eocd.extend_from_slice(&0u16.to_le_bytes()); // disk
        eocd.extend_from_slice(&0u16.to_le_bytes()); // cd disk
        eocd.extend_from_slice(&n.to_le_bytes());
        eocd.extend_from_slice(&n.to_le_bytes());
        eocd.extend_from_slice(&cd_size.to_le_bytes());
        eocd.extend_from_slice(&cd_off.to_le_bytes());
        eocd.extend_from_slice(&0u16.to_le_bytes()); // comment len
        self.out.extend_from_slice(&eocd);
        self.out
    }
}

/// 按名读出 zip 里所有条目（只解 deflate / store；其余方法报错）。
fn read_zip_entries(data: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let u16at = |p: usize| -> u16 { u16::from_le_bytes([data[p], data[p + 1]]) };
    let u32at = |p: usize| -> u32 {
        u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]])
    };
    // 从尾部往前找 EOCD（注释最长 64KB）
    let floor = data.len().saturating_sub(66_000);
    let mut eocd = None;
    let mut i = data.len().saturating_sub(22);
    loop {
        if data.len() >= i + 4 && data[i..i + 4] == SIG_EOCD {
            eocd = Some(i);
            break;
        }
        if i == floor {
            break;
        }
        i -= 1;
    }
    let eocd = eocd.ok_or_else(|| t("cmd.bomxlsx.err.no_eocd"))?;
    let count = u16at(eocd + 10) as usize;
    let mut p = u32at(eocd + 16) as usize;
    let mut out = Vec::new();
    for _ in 0..count {
        if p + 46 > data.len() || data[p..p + 4] != SIG_CD {
            return Err(t("cmd.bomxlsx.err.zip_central_dir"));
        }
        let method = u16at(p + 10);
        let comp_size = u32at(p + 20) as usize;
        let name_len = u16at(p + 28) as usize;
        let extra_len = u16at(p + 30) as usize;
        let comment_len = u16at(p + 32) as usize;
        let local_off = u32at(p + 42) as usize;
        let name = String::from_utf8_lossy(&data[p + 46..p + 46 + name_len]).to_string();
        // 局部头 → 数据起点
        if local_off + 30 > data.len() || data[local_off..local_off + 4] != SIG_LOCAL {
            return Err(t_fmt("cmd.bomxlsx.err.zip_local_header", &[("name", &name)]));
        }
        let l_name = u16at(local_off + 26) as usize;
        let l_extra = u16at(local_off + 28) as usize;
        let dstart = local_off + 30 + l_name + l_extra;
        if dstart + comp_size > data.len() {
            return Err(t_fmt("cmd.bomxlsx.err.zip_out_of_bounds", &[("name", &name)]));
        }
        let raw = &data[dstart..dstart + comp_size];
        let bytes = match method {
            0 => raw.to_vec(),
            8 => {
                use std::io::Read;
                let mut d = flate2::read::DeflateDecoder::new(raw);
                let mut v = Vec::new();
                d.read_to_end(&mut v)
                    .map_err(|e| t_fmt("cmd.bomxlsx.err.unzip", &[("name", &name), ("e", &e.to_string())]))?;
                v
            }
            m => {
                return Err(t_fmt(
                    "cmd.bomxlsx.err.unsupported_method",
                    &[("name", &name), ("m", &m.to_string())],
                ))
            }
        };
        out.push((name, bytes));
        p += 46 + name_len + extra_len + comment_len;
    }
    Ok(out)
}

// ── xlsx 写 ─────────────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            // xlsx/Xml 不允许的控制字符：直接丢掉（不然 Excel 会报文件损坏）
            c if (c as u32) < 0x20 && c != '\t' && c != '\n' && c != '\r' => {}
            c => o.push(c),
        }
    }
    o
}

/// `0 → A`、`25 → Z`、`26 → AA`。
fn col_name(mut i: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (i % 26) as u8);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

/// `A1`/`AA3` → 列号（0 基）。
fn col_index(reference: &str) -> Option<usize> {
    let mut col = 0usize;
    let mut any = false;
    for c in reference.chars() {
        let up = c.to_ascii_uppercase();
        if !up.is_ascii_uppercase() {
            break;
        }
        col = col * 26 + (up as usize - 'A' as usize + 1);
        any = true;
    }
    any.then(|| col - 1)
}

const XML_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;

/// 行 → sheet1.xml（全部用 inlineStr：自包含、免 sharedStrings）。
fn sheet_xml(rows: &[XRow]) -> String {
    let mut s = String::with_capacity(rows.len() * 120 + 512);
    s.push_str(XML_HEAD);
    s.push_str(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    let mut all: Vec<Vec<String>> = Vec::with_capacity(rows.len() + 1);
    all.push(headers(crate::i18n::lang()).iter().map(|s| s.to_string()).collect());
    for r in rows {
        all.push(r.cells.to_vec());
    }
    for (ri, cells) in all.iter().enumerate() {
        let r = ri + 1;
        s.push_str(&format!(r#"<row r="{r}">"#));
        for (ci, v) in cells.iter().enumerate() {
            let t = v.trim();
            if t.is_empty() {
                continue;
            }
            s.push_str(&format!(
                r#"<c r="{}{r}" t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#,
                col_name(ci),
                xml_escape(t)
            ));
        }
        s.push_str("</row>");
    }
    s.push_str("</sheetData></worksheet>");
    s
}

/// 把行写成 `.xlsx` 文件。
pub(crate) fn write_xlsx(path: &Path, rows: &[XRow]) -> Result<(), String> {
    let bytes = xlsx_bytes(rows)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| t_fmt("cmd.bomxlsx.err.make_dir", &[("e", &e.to_string())]))?;
    }
    std::fs::write(path, bytes).map_err(|e| {
        t_fmt(
            "cmd.bom.err.write_failed",
            &[("path", &path.display().to_string()), ("e", &e.to_string())],
        )
    })
}

/// 构造 .xlsx 的字节（zip 容器；`write_xlsx` 与网页「导出→浏览器另存为」共用）。
pub(crate) fn xlsx_bytes(rows: &[XRow]) -> Result<Vec<u8>, String> {
    let mut z = ZipWriter::new();
    z.add(
        "[Content_Types].xml",
        format!(
            r#"{XML_HEAD}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>"#
        )
        .as_bytes(),
    )?;
    z.add(
        "_rels/.rels",
        format!(
            r#"{XML_HEAD}<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#
        )
        .as_bytes(),
    )?;
    z.add(
        "xl/workbook.xml",
        format!(
            r#"{XML_HEAD}<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="明细表" sheetId="1" r:id="rId1"/></sheets></workbook>"#
        )
        .as_bytes(),
    )?;
    z.add(
        "xl/_rels/workbook.xml.rels",
        format!(
            r#"{XML_HEAD}<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#
        )
        .as_bytes(),
    )?;
    z.add(
        "xl/styles.xml",
        format!(
            r#"{XML_HEAD}<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="1"><font><sz val="11"/><name val="宋体"/></font></fonts><fills count="1"><fill><patternFill patternType="none"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs></styleSheet>"#
        )
        .as_bytes(),
    )?;
    z.add("xl/worksheets/sheet1.xml", sheet_xml(rows).as_bytes())?;
    Ok(z.finish())
}

// ── xlsx 读 ─────────────────────────────────────────────────────────────

fn xml_unescape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '&' {
            o.push(c);
            continue;
        }
        let mut ent = String::new();
        while let Some(&n) = it.peek() {
            it.next();
            if n == ';' {
                break;
            }
            ent.push(n);
            if ent.len() > 8 {
                break;
            }
        }
        match ent.as_str() {
            "amp" => o.push('&'),
            "lt" => o.push('<'),
            "gt" => o.push('>'),
            "quot" => o.push('"'),
            "apos" => o.push('\''),
            e if e.starts_with("#x") || e.starts_with("#X") => {
                if let Ok(v) = u32::from_str_radix(&e[2..], 16) {
                    if let Some(ch) = char::from_u32(v) {
                        o.push(ch);
                    }
                }
            }
            e if e.starts_with('#') => {
                if let Ok(v) = e[1..].parse::<u32>() {
                    if let Some(ch) = char::from_u32(v) {
                        o.push(ch);
                    }
                }
            }
            _ => {
                o.push('&');
                o.push_str(&ent);
                o.push(';');
            }
        }
    }
    o
}

/// 找第一个 worksheet part（LibreOffice/Excel/WPS 都写 `xl/worksheets/sheet1.xml`）。
fn first_sheet_path(entries: &[(String, Vec<u8>)]) -> Option<String> {
    if entries.iter().any(|(n, _)| n == "xl/worksheets/sheet1.xml") {
        return Some("xl/worksheets/sheet1.xml".into());
    }
    let mut names: Vec<&String> = entries
        .iter()
        .map(|(n, _)| n)
        .filter(|n| n.starts_with("xl/worksheets/") && n.ends_with(".xml"))
        .collect();
    names.sort();
    names.first().map(|s| (*s).clone())
}

/// 解析 sheet XML → 二维文本（共享字符串表可选）。
fn parse_sheet(xml: &str, shared: &[String]) -> Result<Vec<Vec<String>>, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut r = Reader::from_str(xml);
    r.config_mut().trim_text(false);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<(usize, String)> = Vec::new();
    let mut cur_col: Option<usize> = None;
    let mut cur_type = String::new();
    let mut buf = String::new();
    let mut in_cell = false;
    let mut in_value = false;
    let mut in_is = false;
    loop {
        match r.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_vec();
                match name.as_slice() {
                    b"row" => row = Vec::new(),
                    b"c" => {
                        in_cell = true;
                        cur_type.clear();
                        cur_col = None;
                        buf.clear();
                        for a in e.attributes().flatten() {
                            let k = a.key.as_ref().to_vec();
                            let v = String::from_utf8_lossy(&a.value).to_string();
                            match k.as_slice() {
                                b"r" => cur_col = col_index(&v),
                                b"t" => cur_type = v,
                                _ => {}
                            }
                        }
                    }
                    b"v" if in_cell => {
                        in_value = true;
                        buf.clear();
                    }
                    b"is" if in_cell => in_is = true,
                    b"t" if in_cell => {
                        in_value = true;
                        buf.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                // 空单元格（如 <c r="A1"/>）：占位，值为空
                if e.name().as_ref() == b"c" {
                    let mut col = None;
                    for a in e.attributes().flatten() {
                        if a.key.as_ref() == b"r" {
                            col = col_index(&String::from_utf8_lossy(&a.value));
                        }
                    }
                    if let Some(c) = col {
                        row.push((c, String::new()));
                    }
                }
            }
            Ok(Event::Text(t)) => {
                if in_value {
                    buf.push_str(&t.decode().unwrap_or_default());
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"v" if in_value => {
                    in_value = false;
                    let raw = buf.clone();
                    let val = if cur_type == "s" {
                        raw.trim()
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| shared.get(i).cloned())
                            .unwrap_or_default()
                    } else {
                        raw
                    };
                    let col = cur_col.take().unwrap_or(row.len());
                    // 占位补空（跳号单元格）
                    while row.len() < col {
                        row.push((row.len(), String::new()));
                    }
                    row.push((col, xml_unescape(&val)));
                }
                b"t" if in_value && in_is => {
                    in_value = false;
                }
                b"is" => in_is = false,
                b"c" => {
                    if in_cell && cur_col.is_some() && !buf.is_empty() {
                        let col = cur_col.take().unwrap();
                        while row.len() < col {
                            row.push((row.len(), String::new()));
                        }
                        row.push((col, xml_unescape(&buf.clone())));
                    }
                    in_cell = false;
                    in_value = false;
                }
                b"row" => {
                    let mut cells: Vec<String> = Vec::new();
                    let mut r2 = row.clone();
                    r2.sort_by_key(|(c, _)| *c);
                    for (c, v) in r2 {
                        while cells.len() < c {
                            cells.push(String::new());
                        }
                        cells.push(v);
                    }
                    rows.push(cells);
                    row.clear();
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(t_fmt("cmd.bomxlsx.err.parse_sheet", &[("e", &e.to_string())])),
            _ => {}
        }
    }
    Ok(rows)
}

/// 解析 sharedStrings.xml → 字符串表。
fn parse_shared(xml: &str) -> Vec<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut r = Reader::from_str(xml);
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_si = false;
    let mut in_t = false;
    loop {
        match r.read_event() {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"si" => {
                    in_si = true;
                    cur.clear();
                }
                b"t" if in_si => in_t = true,
                _ => {}
            },
            Ok(Event::Text(t)) if in_t => {
                cur.push_str(&t.decode().unwrap_or_default());
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"t" => in_t = false,
                b"si" if in_si => {
                    in_si = false;
                    out.push(xml_unescape(&cur.clone()));
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    out
}

/// 读 `.xlsx` → 行（跳过表头；列数不足补空）。
pub(crate) fn read_xlsx(path: &Path) -> Result<Vec<XRow>, String> {
    let data = std::fs::read(path).map_err(|e| {
        t_fmt(
            "cmd.bomxlsx.err.read_file",
            &[("path", &path.display().to_string()), ("e", &e.to_string())],
        )
    })?;
    read_xlsx_bytes(&data)
}

/// 读 `.xlsx` 字节 → 行（网页导入用；与 [`read_xlsx`] 同一解析核心）。
pub(crate) fn read_xlsx_bytes(data: &[u8]) -> Result<Vec<XRow>, String> {
    let entries = read_zip_entries(data)?;
    let get = |n: &str| -> Option<&[u8]> {
        entries
            .iter()
            .find(|(name, _)| name == n)
            .map(|(_, d)| d.as_slice())
    };
    let sheet_name = first_sheet_path(&entries).ok_or_else(|| t("cmd.bomxlsx.err.no_sheet"))?;
    let sheet = String::from_utf8_lossy(
        get(&sheet_name).ok_or_else(|| t("cmd.bomxlsx.err.sheet_read"))?,
    )
    .to_string();
    let shared = get("xl/sharedStrings.xml")
        .map(|d| parse_shared(&String::from_utf8_lossy(d)))
        .unwrap_or_default();
    let grid = parse_sheet(&sheet, &shared)?;
    // 丢掉表头行
    let grid: Vec<Vec<String>> = grid
        .into_iter()
        .filter(|r| !(r.first().map(|c| c.trim() == "序号").unwrap_or(false)))
        .collect();
    let mut out = Vec::new();
    for r in grid {
        let mut cells: [String; 9] = Default::default();
        for (i, v) in r.iter().enumerate().take(9) {
            cells[i] = v.trim().to_string();
        }
        if cells.iter().all(|c| c.is_empty()) {
            continue;
        }
        out.push(XRow { cells });
    }
    Ok(out)
}

// ── 导入规划 ────────────────────────────────────────────────────────────

/// 图纸行现状（序号 → (8 格值, 数量锁, 上次导出时的数量基线)）。
pub(crate) type PrevRows = BTreeMap<String, ([String; 8], Option<usize>, Option<usize>)>;

/// 「锁定数量」列 → 锁值。空 = 不锁；数字 = 该值；`Y/是/1/√/true` = 同行数量值。
pub(crate) fn lock_from_cell(cell: &str, qty: Option<usize>) -> Option<usize> {
    let t = cell.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(n) = t.parse::<usize>() {
        return Some(n.max(1));
    }
    if matches!(
        t.to_ascii_lowercase().as_str(),
        "y" | "yes" | "true" | "★" | "✅"
    ) || matches!(t, "是" | "√" | "✓" | "锁")
    {
        return qty;
    }
    None
}

/// 文件行 + 图纸现状 → 目标行规格（**导入的合并规则**见模块文档）。
pub(crate) fn plan_import(prev: &PrevRows, file: &[XRow]) -> Vec<RowSpec> {
    let mut out: Vec<RowSpec> = Vec::new();
    for f in file {
        let no = f.item_no();
        if no.is_empty() {
            continue;
        }
        let cells = f.drawing_cells();
        let prev_row = prev.get(no);
        let mut spec = RowSpec {
            item_no: no.to_string(),
            ..Default::default()
        };
        // 起点：图纸现值（非空即保留）→ 文件非空覆盖
        if let Some((v, lock, exported)) = prev_row {
            spec.code = v[1].clone();
            spec.name = v[2].clone();
            spec.material = v[4].clone();
            spec.unit_weight = v[5].clone();
            spec.remark = v[7].clone();
            spec.qty = v[3].parse().unwrap_or(1);
            spec.lock_qty = *lock;
            spec.exported = *exported;
        }
        // 文件非空单元格覆盖（名称列在图纸里是 名称+规格 合并写的 → 直接整串覆盖）
        if !cells[1].is_empty() {
            spec.code = cells[1].clone();
        }
        if !cells[2].is_empty() {
            spec.name = cells[2].clone();
            spec.spec = String::new();
        }
        if !cells[4].is_empty() {
            spec.material = cells[4].clone();
        }
        if !cells[5].is_empty() {
            spec.unit_weight = cells[5].clone();
        }
        if !cells[7].is_empty() {
            spec.remark = cells[7].clone();
        }
        // 数量：文件的数字优先；再判"手改即锁"与显式锁定列
        let file_qty: Option<usize> = cells[3].parse::<usize>().ok().filter(|q| *q > 0);
        if let Some(q) = file_qty {
            spec.qty = q;
            let changed = spec.exported.map(|b| b != q).unwrap_or(false);
            if changed {
                spec.lock_qty = Some(q); // 手改即锁
            }
        }
        if let Some(lock) = lock_from_cell(&f.cells[8], file_qty.or(Some(spec.qty))) {
            spec.qty = lock;
            spec.lock_qty = Some(lock);
        }
        out.push(spec);
    }
    // 文件里没有的序号：保留图纸那一行
    for (no, (v, lock, exported)) in prev.iter() {
        if out.iter().any(|r| r.item_no == *no) {
            continue;
        }
        let mut r = RowSpec {
            item_no: no.clone(),
            code: v[1].clone(),
            name: v[2].clone(),
            material: v[4].clone(),
            unit_weight: v[5].clone(),
            remark: v[7].clone(),
            qty: v[3].parse().unwrap_or(1),
            lock_qty: *lock,
            exported: *exported,
            ..Default::default()
        };
        r.spec = String::new();
        out.push(r);
    }
    out.sort_by_key(|r| crate::balloon::item_no_key(&r.item_no));
    out
}

/// 从一张图纸行（8 格）建 [`XRow`]（导出用）：第 9 格填锁定值。
pub(crate) fn xrow_from_cells(cells: &[String; 8], lock: Option<usize>) -> XRow {
    XRow {
        cells: [
            cells[0].clone(),
            cells[1].clone(),
            cells[2].clone(),
            cells[3].clone(),
            cells[4].clone(),
            cells[5].clone(),
            cells[6].clone(),
            cells[7].clone(),
            lock.map(|q| q.to_string()).unwrap_or_default(),
        ],
    }
}

/// 需要导出/导入的列名（给报告用）。
pub(crate) fn col_list() -> String {
    HEADERS.join(" | ")
}

/// `CELL_TAGS` 与 xlsx 前 8 列的对应（保证两者没跑偏；中英两套都查）。
pub(crate) fn assert_headers_match() -> bool {
    HEADERS[..8] == CELL_TAGS && HEADERS_EN[..8] == crate::bom::CELL_TAGS_EN
}

// ── 测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn xrow(cells: &[&str; 9]) -> XRow {
        XRow::new(cells.map(|s| s.to_string()))
    }

    #[test]
    fn headers_match_drawing_cells() {
        assert!(assert_headers_match(), "xlsx 前 8 列必须与图纸明细表一致");
        assert_eq!(HEADERS.len(), 9);
        assert_eq!(HEADERS[8], "锁定数量");
        assert_eq!(col_list(), "序号 | 图号 | 名称 | 数量 | 材料 | 单重 | 总重 | 备注 | 锁定数量");
    }

    /// ③ xlsx 表头：**写入随语言**（en 导出英文表头、数据行原样）、**读取两套都认**
    /// （中/英表头行都丢掉）；`assert_headers_match` 中英两套都查。
    #[test]
    fn headers_switch_language_and_parse_accepts_both() {
        use crate::i18n::{set_lang, set_lang_auto, Lang};
        let _g = crate::global_state_test_lock();
        assert!(assert_headers_match(), "中英两套表头都要与 CELL_TAGS 对齐");
        assert_eq!(HEADERS_EN[8], "Locked qty");
        assert_eq!(headers(Lang::Zh), HEADERS);
        assert_eq!(headers(Lang::En), HEADERS_EN);
        let rows = vec![xrow(&[
            "1",
            "GB/T 5782-2016",
            "六角头螺栓 M8x35",
            "2",
            "Q235",
            "0.021",
            "0.042",
            "外购",
            "",
        ])];
        set_lang(Lang::Zh);
        let csv_zh = csv_text(&rows);
        let xml_zh = sheet_xml(&rows);
        set_lang(Lang::En);
        let csv_en = csv_text(&rows);
        let xml_en = sheet_xml(&rows);
        set_lang_auto();
        assert!(
            csv_zh.starts_with("序号,图号,名称,数量,材料,单重,总重,备注,锁定数量"),
            "{csv_zh}"
        );
        assert!(
            csv_en.starts_with("No.,Drawing No.,Name,Qty,Material,Unit wt.,Total wt.,Remarks,Locked qty"),
            "{csv_en}"
        );
        // 数据行两语一致（不翻数据）
        for csv in [&csv_zh, &csv_en] {
            assert!(csv.contains("GB/T 5782-2016") && csv.contains("六角头螺栓 M8x35"), "{csv}");
        }
        assert!(xml_zh.contains("序号") && !xml_zh.contains("Locked qty"), "{xml_zh}");
        assert!(xml_en.contains("Locked qty") && !xml_en.contains("序号"), "{xml_en}");
        // 读取：中/英表头的 CSV 都丢掉表头行 → 同一份数据
        for csv in [csv_zh, csv_en] {
            let back = parse_csv(&csv).unwrap();
            assert_eq!(back.len(), 1, "表头行应被丢掉");
            assert_eq!(back[0].cells[1], "GB/T 5782-2016");
            assert_eq!(back[0].cells[7], "外购");
        }
        // 表头识别：两套的 9 个列头都认，普通数据不算表头
        for h in HEADERS.iter().chain(HEADERS_EN.iter()) {
            assert!(is_header_cell(h), "{h} 应识别为表头");
        }
        assert!(!is_header_cell("GB/T 5782-2016") && !is_header_cell("1"));
    }

    #[test]
    fn csv_roundtrip_with_tricky_fields() {
        let rows = vec![
            xrow(&["1", "GB/T 5782-2016", "六角头螺栓 M8x35", "2", "Q235", "0.021", "0.042", "带\"垫圈\",外购", ""]),
            xrow(&["A1", "0165", "偏心轴 φ12", "1", "45", "1.35", "1.35", "换行\n备注", "3"]),
        ];
        let text = csv_text(&rows);
        let back = parse_csv(&text).unwrap();
        assert_eq!(back, rows);
        // 表头识别 + BOM 容错
        let with_bom = format!("\u{feff}{text}");
        assert_eq!(parse_csv(&with_bom).unwrap(), rows);
        // 列数过多 → 报错
        assert!(parse_csv("1,2,3,4,5,6,7,8,9,10\n").is_err());
    }

    #[test]
    fn col_letters() {
        assert_eq!(col_name(0), "A");
        assert_eq!(col_name(8), "I");
        assert_eq!(col_name(25), "Z");
        assert_eq!(col_name(26), "AA");
        assert_eq!(col_index("A1"), Some(0));
        assert_eq!(col_index("I12"), Some(8));
        assert_eq!(col_index("AA3"), Some(26));
    }

    #[test]
    fn xlsx_write_then_read_roundtrip() {
        let dir = std::env::temp_dir().join("ocs-ocsm-xlsx-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rt.xlsx");
        let rows = vec![
            xrow(&["1", "GB/T 5782-2016", "六角头螺栓 M8x35", "2", "Q235", "0.021", "0.042", "外购", "2"]),
            xrow(&["A1", "0165", "偏心轴 φ12", "1", "45", "1.35", "1.35", "", ""]),
        ];
        write_xlsx(&path, &rows).unwrap();
        // 必须是 zip（PK 头）
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..2], b"PK");
        let back = read_xlsx(&path).unwrap();
        assert_eq!(back, rows, "写→读 必须一模一样");
        // 再读一次覆盖写（幂等）
        write_xlsx(&path, &back).unwrap();
        assert_eq!(read_xlsx(&path).unwrap(), rows);
    }

    #[test]
    fn read_xlsx_survives_shared_strings_and_sparse_cells() {
        // 模拟 LibreOffice/Excel 风格：sharedStrings + 跳号单元格 + 数字单元格
        let dir = std::env::temp_dir().join("ocs-ocsm-xlsx-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shared.xlsx");
        let sheet = format!(
            r#"{XML_HEAD}<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row><row r="2"><c r="A2" t="s"><v>2</v></c><c r="D2"><v>3</v></c></row></sheetData></worksheet>"#
        );
        let shared = format!(
            r#"{XML_HEAD}<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="3" uniqueCount="3"><si><t>序号</t></si><si><t>图号</t></si><si><t>A1</t></si></sst>"#
        );
        let mut z = ZipWriter::new();
        z.add("xl/worksheets/sheet1.xml", sheet.as_bytes()).unwrap();
        z.add("xl/sharedStrings.xml", shared.as_bytes()).unwrap();
        std::fs::write(&path, z.finish()).unwrap();
        let rows = read_xlsx(&path).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].item_no(), "A1");
        assert_eq!(rows[0].cells[3], "3"); // 数字单元格 → 文本
        assert_eq!(rows[0].cells[1], ""); // 跳号补空
    }

    #[test]
    fn lock_cell_forms() {
        assert_eq!(lock_from_cell("", Some(2)), None);
        assert_eq!(lock_from_cell(" Y ", Some(2)), Some(2));
        assert_eq!(lock_from_cell("是", Some(5)), Some(5));
        assert_eq!(lock_from_cell("1", Some(5)), Some(1));
        assert_eq!(lock_from_cell("7", Some(5)), Some(7));
        assert_eq!(lock_from_cell("随便写", Some(5)), None);
    }

    #[test]
    fn import_applies_hand_edit_lock_and_keeps_blank_cells() {
        let mut prev: PrevRows = BTreeMap::new();
        prev.insert(
            "5".to_string(),
            (
                [
                    "5".into(),
                    "GB/T 5782-2016".into(),
                    "六角头螺栓 M8x35".into(),
                    "2".into(),
                    "Q235".into(),
                    "0.021".into(),
                    "0.042".into(),
                    "".into(),
                ],
                None,
                Some(2),
            ),
        );
        // 文件：数量手改成 3（无锁定列）；名称列留空（不该擦掉图纸里的名称）
        let file = vec![xrow(&["5", "", "", "3", "", "", "", "", ""])];
        let rows = plan_import(&prev, &file);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].qty, 3);
        assert_eq!(rows[0].lock_qty, Some(3), "手改即锁");
        assert_eq!(rows[0].name, "六角头螺栓 M8x35", "空白格保留图纸值");
        assert_eq!(rows[0].code, "GB/T 5782-2016");
        // 文件：备注非空 → 覆盖；数量与基线一致 → 不锁
        let file2 = vec![xrow(&["5", "", "", "2", "", "", "", "外购", ""])];
        let rows2 = plan_import(&prev, &file2);
        assert_eq!(rows2[0].remark, "外购");
        assert!(rows2[0].lock_qty.is_none(), "数量没变 → 不锁");
    }

    #[test]
    fn import_keeps_rows_missing_from_the_file() {
        let mut prev: PrevRows = BTreeMap::new();
        prev.insert(
            "5".to_string(),
            (["5".into(), "A".into(), "螺栓".into(), "2".into(), "".into(), "".into(), "".into(), "".into()], None, None),
        );
        prev.insert(
            "6".to_string(),
            (["6".into(), "B".into(), "螺母".into(), "4".into(), "".into(), "".into(), "".into(), "".into()], None, None),
        );
        let file = vec![xrow(&["5", "A", "螺栓", "2", "", "", "", "", ""])];
        let rows = plan_import(&prev, &file);
        assert_eq!(rows.len(), 2, "文件里没有的序号要保留");
        assert_eq!(rows[1].item_no, "6");
        assert_eq!(rows[1].qty, 4);
    }

    #[test]
    fn import_honours_explicit_lock_column() {
        let prev: PrevRows = BTreeMap::new();
        let file = vec![xrow(&["7", "X", "垫圈", "2", "Q235", "0.01", "", "", "9"])];
        let rows = plan_import(&prev, &file);
        assert_eq!(rows[0].qty, 9);
        assert_eq!(rows[0].lock_qty, Some(9));
        // 字母锁 = 锁为同行数量
        let file2 = vec![xrow(&["7", "X", "垫圈", "2", "Q235", "0.01", "", "", "Y"])];
        let rows2 = plan_import(&prev, &file2);
        assert_eq!(rows2[0].qty, 2);
        assert_eq!(rows2[0].lock_qty, Some(2));
    }

    #[test]
    fn total_weight_column_is_ignored_on_import() {
        let prev: PrevRows = BTreeMap::new();
        let file = vec![xrow(&["1", "X", "件", "2", "45", "1.5", "999", "", ""])];
        let rows = plan_import(&prev, &file);
        // 总重不接输入 → RowSpec 里没有该字段，values() 会按 单重×数量 算
        assert_eq!(rows[0].unit_weight, "1.5");
        assert_eq!(rows[0].qty, 2);
    }

    #[test]
    fn csv_export_text_is_excel_friendly() {
        // 导出成 CSV 时外面会加 BOM（Excel 认中文）；这里验证文本本身与解析对称
        let rows = vec![xrow(&["1", "GB/T 5782", "螺栓", "2", "Q235", "0.02", "0.04", "外购", "2"])];
        let text = csv_text(&rows);
        assert!(text.starts_with("序号,"), "首行是表头");
        assert!(text.contains("GB/T 5782"), "不含逗号的字段不加引号");
        assert!(text.contains("\r\n"), "CRLF 行尾");
        let back = parse_csv(&format!("\u{feff}{text}")).unwrap();
        assert_eq!(back, rows);
    }
}
