//! **矩形花键**（GB/T 1144 规格代号）参数化画法 —— 独立要素与轴段特征共用一套几何。
//!
//! # 数据出处
//!
//! * 规格代号 `N x d x D x B`（齿数 × 小径 × 大径 × 键宽）**自带全部被加工件尺寸**，
//!   按代号解析即可，不需要查表；出处：GB/T 1144-2001《矩形花键 尺寸、公差和检验》。
//! * **滚刀外径 `de` 查表**：GB/T 10952-2005《矩形花键滚刀》表 1（轻系列 15 行）
//!   + 表 2（中系列 18 行）；数据抄自用户调查
//!   `~/桌面/OCSM/review/花键滚刀资料/花键滚刀调查.md`（同目录有 GB 原文页 JPG 可对照）。
//!   `de/2` 允许用参数覆盖（`de 63`）。
//!
//! # 画法（唯一权威 = 用户模板 `~/桌面/OCSM/review/矩形花键.dxf`，逐图元反解）
//!
//! ## 正视图（端视图；原点 = 两中心线交点）
//!
//! * **相位（用户定案）**：齿槽心线 = `0° + k·(360/N)`，齿心线 = `180/N + k·(360/N)`
//!   （N=6 → 齿心线 30°/90°/150°…，模板 6 齿即此相位）；
//! * 齿顶弧 `R=D/2`，每齿绕齿心线两侧各 `asin(B/D)`；
//! * 齿根弧 `R=d/2`，每槽绕槽心线两侧各 `180/N − asin(B/d)`；
//! * 每条齿侧 = 与齿心线**平行、距 `B/2`** 的直线段：从与 `R=d/2` 的交点到与
//!   `R=D/2` 的交点（模板例 6×23×26×6 正上齿 `(±3, 11.101802)→(±3, 12.649111)`）；
//! * 无倒角/圆角、无分度圆、无虚线；两中心线各 `±(D+6)/2`（模板 D=26 → ±16）。
//!
//! ## 常规侧视图（矩形 L×D）
//!
//! 端面线 `x=0 / x=L`（高 `±D/2`）+ 大径线 `y=±D/2` + **小径线 `y=±d/2` 用
//! `2细线层` 贯通全长** + 中心线（长 `L+6`）。**无收尾弧**（收尾只属于轴段特征）。
//!
//! ## 侧剖视图
//!
//! 轮廓同常规侧视图，但**小径线改 `1轮廓实线层`**，并在
//! `5剖面线层` 上加 ANSI31（比例 1.0）：**只填「轴线↔小径」两条带**
//! （y 0→±d/2，x 0→L），**不填齿部**；一个 HATCH、2 个边界环、每环 4 条 LineEdge、
//! `flags = external|outermost`（与模板一致）。
//!
//! ## 轴段特征（`SPLINE` / XL 独立插入以外，见 `shaft.rs`）
//!
//! 局部系 `x′=0` = 段左端面、轴线 = x 轴：
//!
//! * 大径线 `y=±D/2`；小径细线 `y=±d/2` 落 `2细线层`，从 `x′=0` 到 `x′=L`（L = 满齿段长）；
//! * **收尾弧**：`R = de/2`，圆心 `(L, ±(d/2 + R))`（正上/正下），与小径相切
//!   （切点 = `(L, ±d/2)`），弧向右延伸至与大径相交（不相切，模板交角 17.75°），
//!   末端 `x′ = L + l`，`l = √(h(2R−h))`、`h = (D−d)/2`；
//!   模板核对：6×23×26×6 → de=63、R=31.5、h=1.5、**l=9.6047**（模板实测 ✓）；
//! * 收尾起点 / 终点各一条细竖线（`2细线层`）跨 `y=±D/2`（模板两根都在）；
//! * **不自动画引入倒角**（用户定案：由用户自己在相邻段写 `CH`；模板的 φ22→φ26
//!   倒角就是用户自己的表达）。
//!
//! 轴段长度口径：`SPLINE … L30` 的 **L 是满齿段长**，段长 = `L + l`（收尾段计入本段）。

use ocs_plugin_api::host::acadrust::entities::EntityType;

use crate::partgen_kit::{
    arc, hatch_ansi31_rings, line, trim, HatchEdge, LAYER_CENTER, LAYER_MAIN, LAYER_THIN,
};

/// 被加工件标准号。
pub const CODE: &str = "GB/T 1144-2001";
/// 滚刀（de）数据来源标准号。
pub const HOB_CODE: &str = "GB/T 10952-2005";

/// 一行规格：`N×d×D×B` + 滚刀外径 `de`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplineSpec {
    /// 规格代号（标准写法，小写 x）。
    pub code: &'static str,
    /// 齿数 N。
    pub n: u32,
    /// 小径 d。
    pub d_minor: f64,
    /// 大径 D。
    pub d_major: f64,
    /// 键宽（齿厚）B。
    pub b: f64,
    /// 滚刀外径 de（GB/T 10952-2005 表 1/表 2）。
    pub de: f64,
    /// 系列（轻/中）。
    pub series: &'static str,
}

/// **GB/T 10952-2005 表 1（轻系列，15 行）+ 表 2（中系列，18 行）唯一数据源**。
///
/// 出处：用户调查 `花键滚刀资料/花键滚刀调查.md` 第 2.1 节
/// （同目录 `GB-T10952-2005_p03_表1轻系列_表2中系列头.jpg` / `p04_表2续` 原图可对照）。
/// 表 2 注：`6×11×14×3`、`6×13×16×3.5` 不宜展成滚切加工，标准未列入。
pub const SPLINE_SPECS: &[SplineSpec] = &[
    // ── 表 1 轻系列（15 行）──
    SplineSpec {
        code: "6x23x26x6",
        n: 6,
        d_minor: 23.0,
        d_major: 26.0,
        b: 6.0,
        de: 63.0,
        series: "轻",
    },
    SplineSpec {
        code: "6x26x30x6",
        n: 6,
        d_minor: 26.0,
        d_major: 30.0,
        b: 6.0,
        de: 63.0,
        series: "轻",
    },
    SplineSpec {
        code: "6x28x32x7",
        n: 6,
        d_minor: 28.0,
        d_major: 32.0,
        b: 7.0,
        de: 71.0,
        series: "轻",
    },
    SplineSpec {
        code: "8x32x36x6",
        n: 8,
        d_minor: 32.0,
        d_major: 36.0,
        b: 6.0,
        de: 71.0,
        series: "轻",
    },
    SplineSpec {
        code: "8x36x40x7",
        n: 8,
        d_minor: 36.0,
        d_major: 40.0,
        b: 7.0,
        de: 80.0,
        series: "轻",
    },
    SplineSpec {
        code: "8x42x46x8",
        n: 8,
        d_minor: 42.0,
        d_major: 46.0,
        b: 8.0,
        de: 80.0,
        series: "轻",
    },
    SplineSpec {
        code: "8x46x50x9",
        n: 8,
        d_minor: 46.0,
        d_major: 50.0,
        b: 9.0,
        de: 90.0,
        series: "轻",
    },
    SplineSpec {
        code: "8x52x58x10",
        n: 8,
        d_minor: 52.0,
        d_major: 58.0,
        b: 10.0,
        de: 90.0,
        series: "轻",
    },
    SplineSpec {
        code: "8x56x62x10",
        n: 8,
        d_minor: 56.0,
        d_major: 62.0,
        b: 10.0,
        de: 100.0,
        series: "轻",
    },
    SplineSpec {
        code: "8x62x68x12",
        n: 8,
        d_minor: 62.0,
        d_major: 68.0,
        b: 12.0,
        de: 100.0,
        series: "轻",
    },
    SplineSpec {
        code: "10x72x78x12",
        n: 10,
        d_minor: 72.0,
        d_major: 78.0,
        b: 12.0,
        de: 100.0,
        series: "轻",
    },
    SplineSpec {
        code: "10x82x88x12",
        n: 10,
        d_minor: 82.0,
        d_major: 88.0,
        b: 12.0,
        de: 112.0,
        series: "轻",
    },
    SplineSpec {
        code: "10x92x98x14",
        n: 10,
        d_minor: 92.0,
        d_major: 98.0,
        b: 14.0,
        de: 112.0,
        series: "轻",
    },
    SplineSpec {
        code: "10x102x108x16",
        n: 10,
        d_minor: 102.0,
        d_major: 108.0,
        b: 16.0,
        de: 112.0,
        series: "轻",
    },
    SplineSpec {
        code: "10x112x120x18",
        n: 10,
        d_minor: 112.0,
        d_major: 120.0,
        b: 18.0,
        de: 118.0,
        series: "轻",
    },
    // ── 表 2 中系列（18 行）──
    SplineSpec {
        code: "6x16x20x4",
        n: 6,
        d_minor: 16.0,
        d_major: 20.0,
        b: 4.0,
        de: 63.0,
        series: "中",
    },
    SplineSpec {
        code: "6x18x22x5",
        n: 6,
        d_minor: 18.0,
        d_major: 22.0,
        b: 5.0,
        de: 63.0,
        series: "中",
    },
    SplineSpec {
        code: "6x21x25x5",
        n: 6,
        d_minor: 21.0,
        d_major: 25.0,
        b: 5.0,
        de: 71.0,
        series: "中",
    },
    SplineSpec {
        code: "6x23x28x6",
        n: 6,
        d_minor: 23.0,
        d_major: 28.0,
        b: 6.0,
        de: 71.0,
        series: "中",
    },
    SplineSpec {
        code: "6x26x32x6",
        n: 6,
        d_minor: 26.0,
        d_major: 32.0,
        b: 6.0,
        de: 80.0,
        series: "中",
    },
    SplineSpec {
        code: "6x28x34x7",
        n: 6,
        d_minor: 28.0,
        d_major: 34.0,
        b: 7.0,
        de: 80.0,
        series: "中",
    },
    SplineSpec {
        code: "8x32x38x6",
        n: 8,
        d_minor: 32.0,
        d_major: 38.0,
        b: 6.0,
        de: 80.0,
        series: "中",
    },
    SplineSpec {
        code: "8x36x42x7",
        n: 8,
        d_minor: 36.0,
        d_major: 42.0,
        b: 7.0,
        de: 90.0,
        series: "中",
    },
    SplineSpec {
        code: "8x42x48x8",
        n: 8,
        d_minor: 42.0,
        d_major: 48.0,
        b: 8.0,
        de: 90.0,
        series: "中",
    },
    SplineSpec {
        code: "8x46x54x9",
        n: 8,
        d_minor: 46.0,
        d_major: 54.0,
        b: 9.0,
        de: 90.0,
        series: "中",
    },
    SplineSpec {
        code: "8x52x60x10",
        n: 8,
        d_minor: 52.0,
        d_major: 60.0,
        b: 10.0,
        de: 90.0,
        series: "中",
    },
    SplineSpec {
        code: "8x56x65x10",
        n: 8,
        d_minor: 56.0,
        d_major: 65.0,
        b: 10.0,
        de: 100.0,
        series: "中",
    },
    SplineSpec {
        code: "8x62x72x12",
        n: 8,
        d_minor: 62.0,
        d_major: 72.0,
        b: 12.0,
        de: 112.0,
        series: "中",
    },
    SplineSpec {
        code: "10x72x82x12",
        n: 10,
        d_minor: 72.0,
        d_major: 82.0,
        b: 12.0,
        de: 112.0,
        series: "中",
    },
    SplineSpec {
        code: "10x82x92x12",
        n: 10,
        d_minor: 82.0,
        d_major: 92.0,
        b: 12.0,
        de: 112.0,
        series: "中",
    },
    SplineSpec {
        code: "10x92x102x14",
        n: 10,
        d_minor: 92.0,
        d_major: 102.0,
        b: 14.0,
        de: 118.0,
        series: "中",
    },
    SplineSpec {
        code: "10x102x112x16",
        n: 10,
        d_minor: 102.0,
        d_major: 112.0,
        b: 16.0,
        de: 118.0,
        series: "中",
    },
    SplineSpec {
        code: "10x112x125x18",
        n: 10,
        d_minor: 112.0,
        d_major: 125.0,
        b: 18.0,
        de: 125.0,
        series: "中",
    },
];

/// 规格代号 → 四个数字（`6x23x26x6`；`×`/`X`/空格/`*` 都收）。
///
/// **按代号解析、不查表**（代号自带 N/d/D/B）；`de` 另用 [`lookup_de`] 查。
pub fn parse_code(code: &str) -> Result<(u32, f64, f64, f64), String> {
    let text = code.trim();
    let parts: Vec<&str> = text
        .split(['x', 'X', '×', '*', '＊', ' '])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() != 4 {
        return Err(format!(
            "矩形花键规格代号「{code}」不是 N×d×D×B（例 6x23x26x6），收到 {} 段",
            parts.len()
        ));
    }
    let n: u32 = parts[0]
        .parse()
        .map_err(|_| format!("矩形花键规格代号「{code}」的齿数 N 不是正整数"))?;
    let d: f64 = parts[1]
        .parse()
        .map_err(|_| format!("矩形花键规格代号「{code}」的小径 d 不是数字"))?;
    let big: f64 = parts[2]
        .parse()
        .map_err(|_| format!("矩形花键规格代号「{code}」的大径 D 不是数字"))?;
    let b: f64 = parts[3]
        .parse()
        .map_err(|_| format!("矩形花键规格代号「{code}」的键宽 B 不是数字"))?;
    validate_dims(n, d, big, b)?;
    Ok((n, d, big, b))
}

/// N/d/D/B → 表里的规格（轻系列优先；1e-9 容差，找不到返回 None）。
pub fn lookup_spec(n: u32, d: f64, big: f64, b: f64) -> Option<&'static SplineSpec> {
    SPLINE_SPECS.iter().find(|s| {
        s.n == n
            && (s.d_minor - d).abs() < 1e-9
            && (s.d_major - big).abs() < 1e-9
            && (s.b - b).abs() < 1e-9
    })
}

/// N/d/D/B → 滚刀外径 `de`（查表；找不到 None，调用方报「请给 de」）。
pub fn lookup_de(n: u32, d: f64, big: f64, b: f64) -> Option<f64> {
    lookup_spec(n, d, big, b).map(|s| s.de)
}

/// 几何合法性的公共校验（代号、独立要素、轴段三处共用）。
pub fn validate_dims(n: u32, d: f64, big: f64, b: f64) -> Result<(), String> {
    if !(3..=100).contains(&n) {
        return Err(format!("矩形花键：齿数 N={n} 非法（3..=100）"));
    }
    for (name, value) in [("小径 d", d), ("大径 D", big), ("键宽 B", b)] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("矩形花键：{name}={value} 必须是正数"));
        }
    }
    if big <= d {
        return Err(format!(
            "矩形花键：大径 D={} 必须大于小径 d={}",
            trim(big),
            trim(d)
        ));
    }
    if b >= d {
        return Err(format!(
            "矩形花键：键宽 B={} 必须小于小径 d={}（齿侧与齿根弧要交得上）",
            trim(b),
            trim(d)
        ));
    }
    if b >= big {
        return Err(format!(
            "矩形花键：键宽 B={} 必须小于大径 D={}",
            trim(b),
            trim(big)
        ));
    }
    Ok(())
}

/// 一个已定参数的矩形花键（规格 + 可选 de 覆盖 + 可选 L）。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct RectSpline {
    /// 齿数 N。
    pub n: u32,
    /// 小径 d。
    pub d: f64,
    /// 大径 D。
    pub big: f64,
    /// 键宽 B。
    pub b: f64,
    /// 滚刀外径 de（查表值或覆盖值）。
    pub de: f64,
    /// 满齿段长 L（正视图不用；= 0 表示「未给」）。
    pub len: f64,
}

impl RectSpline {
    /// 由 N/d/D/B + de 构造并校验（`de` 必须大于大径 D，否则收尾弧无交点）。
    pub fn new(n: u32, d: f64, big: f64, b: f64, de: f64, len: f64) -> Result<Self, String> {
        validate_dims(n, d, big, b)?;
        if !de.is_finite() || de <= 0.0 {
            return Err(format!("矩形花键：de={de} 必须是正数"));
        }
        if de <= big + 1e-9 {
            return Err(format!(
                "矩形花键：滚刀外径 de={} 必须大于大径 D={}（否则收尾弧切不出去）",
                trim(de),
                trim(big)
            ));
        }
        if !len.is_finite() || len < 0.0 {
            return Err(format!("矩形花键：段长 L={len} 不能为负"));
        }
        Ok(Self {
            n,
            d,
            big,
            b,
            de,
            len,
        })
    }

    /// 由规格代号构造；`de_override = None` 查 GB/T 10952 表（查不到报错并要求给 de）。
    pub fn from_code(code: &str, de_override: Option<f64>, len: f64) -> Result<Self, String> {
        let (n, d, big, b) = parse_code(code)?;
        let de = match de_override {
            Some(de) => de,
            None => lookup_de(n, d, big, b).ok_or_else(|| {
                let list = SPLINE_SPECS
                    .iter()
                    .take(3)
                    .map(|s| s.code)
                    .collect::<Vec<_>>()
                    .join("、");
                format!(
                    "矩形花键：规格 {code} 不在 GB/T 10952-2005 表 1/表 2 里，de 查不到 —— \
                     请给 de 覆盖（例 `de 63`）。表内规格如 {list} …（共 {} 条）",
                    SPLINE_SPECS.len()
                )
            })?,
        };
        Self::new(n, d, big, b, de, len)
    }

    /// 规格代号（规范小写 x）。
    pub fn code(&self) -> String {
        format!(
            "{}x{}x{}x{}",
            self.n,
            trim(self.d),
            trim(self.big),
            trim(self.b)
        )
    }

    /// 小径半径 r = d/2。
    pub fn minor_radius(&self) -> f64 {
        self.d / 2.0
    }

    /// 大径半径 R = D/2。
    pub fn major_radius(&self) -> f64 {
        self.big / 2.0
    }

    /// 齿深 h = (D−d)/2。
    pub fn depth(&self) -> f64 {
        (self.big - self.d) / 2.0
    }

    /// 滚刀半径 R_hob = de/2。
    pub fn hob_radius(&self) -> f64 {
        self.de / 2.0
    }

    /// **收尾长度** `l = √(h(2R−h))`（R = de/2；模板核对 6×23×26×6 → 9.6047）。
    pub fn runout(&self) -> f64 {
        let h = self.depth();
        let r = self.hob_radius();
        (h * (2.0 * r - h)).max(0.0).sqrt()
    }

    /// 轴段长度 = 满齿段长 L + 收尾段 l。
    pub fn segment_len(&self) -> f64 {
        self.len + self.runout()
    }

    /// 收尾弧在**大径交点**处的极角（相对弧圆心，度；模板 6×23×26×6 → 72.2472°）。
    ///
    /// 上半弧从 270° 转到 `360° − a`；下半弧从 `a` 转到 90°。
    pub fn runout_end_angle(&self) -> f64 {
        let h = self.depth();
        let r = self.hob_radius();
        let l = self.runout();
        (r - h).atan2(l).to_degrees()
    }

    // ── 视图 ────────────────────────────────────────────────────────────

    /// 正视图（端视图齿形）+ 十字中心线；模板核对 26 条图元。
    pub fn front_view(&self) -> Vec<EntityType> {
        let (ra, rf) = (self.major_radius(), self.minor_radius());
        let (half_b, n) = (self.b / 2.0, self.n as f64);
        let step = 360.0 / n;
        // 齿侧与齿顶/齿根圆的交点（半长 = √(r²−(B/2)²)）。
        let t_tip = (ra * ra - half_b * half_b).sqrt();
        let t_root = (rf * rf - half_b * half_b).sqrt();
        let alpha_tip = (half_b / ra).asin().to_degrees();
        let alpha_root = 180.0 / n - (half_b / rf).asin().to_degrees();
        let mut out = Vec::with_capacity((self.n as usize) * 3 + (self.n as usize) + 2);

        // 相位：齿槽心线 = 0°+k·step，齿心线 = 90°+k·step（模板首齿在正上方）。
        for k in 0..self.n {
            let theta = (90.0 + step * k as f64).to_radians();
            let (u, v) = ((theta.cos(), theta.sin()), (-theta.sin(), theta.cos()));
            // 每条齿侧：距齿心线 B/2 的平行线段（先 −v 侧再 +v 侧，与模板序一致）。
            for side in [-1.0_f64, 1.0] {
                let off = side * half_b;
                let a = [t_root * u.0 + off * v.0, t_root * u.1 + off * v.1];
                let b = [t_tip * u.0 + off * v.0, t_tip * u.1 + off * v.1];
                out.push(line(a, b, LAYER_MAIN));
            }
            let c = 90.0 + step * k as f64;
            out.push(arc(
                [0.0, 0.0],
                ra,
                c - alpha_tip,
                c + alpha_tip,
                LAYER_MAIN,
            ));
        }
        // 齿根弧（槽心线两侧各 alpha_root）。
        for k in 0..self.n {
            let c = step * k as f64;
            out.push(arc(
                [0.0, 0.0],
                rf,
                c - alpha_root,
                c + alpha_root,
                LAYER_MAIN,
            ));
        }
        // 十字中心线：长度 = D + 6（模板 ±16）。
        let half = ra + 3.0;
        out.push(line([-half, 0.0], [half, 0.0], LAYER_CENTER));
        out.push(line([0.0, -half], [0.0, half], LAYER_CENTER));
        out
    }

    /// 常规侧视图（矩形 L×D + 小径细线 + 中心线）。
    pub fn side_view(&self, len: f64) -> Vec<EntityType> {
        let (ra, rf) = (self.major_radius(), self.minor_radius());
        let mut out = Vec::with_capacity(7);
        // 端面线 x=0 / x=L（模板顺序：左端面 → 右端面 → 上/下大径 → 下/上小径 → 中心线）
        out.push(line([0.0, -ra], [0.0, ra], LAYER_MAIN));
        out.push(line([len, -ra], [len, ra], LAYER_MAIN));
        out.push(line([0.0, ra], [len, ra], LAYER_MAIN));
        out.push(line([0.0, -ra], [len, -ra], LAYER_MAIN));
        out.push(line([0.0, -rf], [len, -rf], LAYER_THIN));
        out.push(line([0.0, rf], [len, rf], LAYER_THIN));
        out.push(line([-3.0, 0.0], [len + 3.0, 0.0], LAYER_CENTER));
        out
    }

    /// 侧剖视图（小径线改 `1轮廓实线层` + 轴线↔小径两条带 ANSI31）。
    pub fn section_view(&self, len: f64) -> Vec<EntityType> {
        let (ra, rf) = (self.major_radius(), self.minor_radius());
        let mut out = Vec::with_capacity(9);
        out.push(line([0.0, -ra], [0.0, ra], LAYER_MAIN));
        out.push(line([len, -ra], [len, ra], LAYER_MAIN));
        out.push(line([0.0, ra], [len, ra], LAYER_MAIN));
        out.push(line([0.0, -ra], [len, -ra], LAYER_MAIN));
        out.push(line([0.0, -rf], [len, -rf], LAYER_MAIN));
        out.push(line([0.0, rf], [len, rf], LAYER_MAIN));
        out.push(line([-3.0, 0.0], [len + 3.0, 0.0], LAYER_CENTER));
        // 剖面线：只填「轴线↔小径」两条带，不填齿部（模板 2 环 × 4 LineEdge）。
        let upper = vec![
            HatchEdge::Line {
                a: [0.0, 0.0],
                b: [len, 0.0],
            },
            HatchEdge::Line {
                a: [len, 0.0],
                b: [len, rf],
            },
            HatchEdge::Line {
                a: [len, rf],
                b: [0.0, rf],
            },
            HatchEdge::Line {
                a: [0.0, rf],
                b: [0.0, 0.0],
            },
        ];
        let lower = vec![
            HatchEdge::Line {
                a: [0.0, 0.0],
                b: [0.0, -rf],
            },
            HatchEdge::Line {
                a: [0.0, -rf],
                b: [len, -rf],
            },
            HatchEdge::Line {
                a: [len, -rf],
                b: [len, 0.0],
            },
            HatchEdge::Line {
                a: [len, 0.0],
                b: [0.0, 0.0],
            },
        ];
        out.push(hatch_ansi31_rings(&[upper, lower], 0.0, crate::gear::HATCH_PATTERN_SCALE));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocs_plugin_api::host::acadrust::entities::hatch::BoundaryEdge;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn has_line(entities: &[EntityType], a: [f64; 2], b: [f64; 2]) -> bool {
        entities.iter().any(|e| match e {
            EntityType::Line(l) => {
                let (p, q) = ([l.start.x, l.start.y], [l.end.x, l.end.y]);
                (near(p[0], a[0]) && near(p[1], a[1]) && near(q[0], b[0]) && near(q[1], b[1]))
                    || (near(p[0], b[0])
                        && near(p[1], b[1])
                        && near(q[0], a[0])
                        && near(q[1], a[1]))
            }
            _ => false,
        })
    }

    fn has_arc(entities: &[EntityType], c: [f64; 2], r: f64, a0: f64, a1: f64) -> bool {
        entities.iter().any(|e| match e {
            EntityType::Arc(a) => {
                near(a.center.x, c[0])
                    && near(a.center.y, c[1])
                    && near(a.radius, r)
                    && near(a.start_angle.to_degrees(), a0)
                    && near(a.end_angle.to_degrees(), a1)
            }
            _ => false,
        })
    }

    /// 规格代号解析：`x`/`×` 都收；de 查表（6×23×26×6 → 63）。
    #[test]
    fn parse_code_and_de_lookup() {
        assert_eq!(parse_code("6x23x26x6").unwrap(), (6, 23.0, 26.0, 6.0));
        assert_eq!(parse_code("6×23×26×6").unwrap(), (6, 23.0, 26.0, 6.0));
        assert_eq!(
            parse_code(" 10x112x125x18 ").unwrap(),
            (10, 112.0, 125.0, 18.0)
        );
        assert!(parse_code("6x23x26").is_err());
        assert!(parse_code("6x23x26x6x1").is_err());
        assert!(parse_code("x23x26x6").is_err());
        assert_eq!(lookup_de(6, 23.0, 26.0, 6.0), Some(63.0));
        assert_eq!(lookup_de(10, 112.0, 125.0, 18.0), Some(125.0));
        assert_eq!(lookup_de(6, 11.0, 14.0, 3.0), None);
        assert_eq!(SPLINE_SPECS.len(), 33, "轻 15 + 中 18");
        assert_eq!(SPLINE_SPECS.iter().filter(|s| s.series == "轻").count(), 15);
        assert_eq!(SPLINE_SPECS.iter().filter(|s| s.series == "中").count(), 18);
    }

    /// `from_code`：de 查表 / 覆盖 / 表外规格要求 de；非法几何报错。
    #[test]
    fn from_code_and_overrides() {
        let sp = RectSpline::from_code("6x23x26x6", None, 30.0).unwrap();
        assert!(near(sp.de, 63.0));
        assert!(near(sp.runout(), 9.604_687), "{}", sp.runout());
        let sp = RectSpline::from_code("6x23x26x6", Some(71.0), 30.0).unwrap();
        assert!(near(sp.de, 71.0));
        // 表外规格：查不到 de 要报错并指路；给了 de 就能用
        let err = RectSpline::from_code("6x11x14x3", None, 20.0).unwrap_err();
        assert!(err.contains("de"), "{err}");
        assert!(RectSpline::from_code("6x11x14x3", Some(63.0), 20.0).is_ok());
        // de ≤ D 报错；D ≤ d / B ≥ d 报错
        assert!(RectSpline::from_code("6x23x26x6", Some(26.0), 30.0).is_err());
        assert!(RectSpline::new(6, 26.0, 23.0, 6.0, 63.0, 30.0).is_err());
        assert!(RectSpline::new(6, 23.0, 26.0, 23.0, 63.0, 30.0).is_err());
    }

    /// 正视图：26 条图元、齿侧坐标（模板正上齿）、弧角（齿顶/齿根跨度）、相位、中心线。
    #[test]
    fn front_view_matches_template() {
        let sp = RectSpline::from_code("6x23x26x6", None, 30.0).unwrap();
        let e = sp.front_view();
        assert_eq!(e.len(), 26, "6 齿 × 3 + 6 齿根弧 + 2 中心线");
        // 正上齿两侧：模板 (±3, 11.101802) → (±3, 12.649111)
        assert!(has_line(&e, [3.0, 11.101_802], [3.0, 12.649_111]));
        assert!(has_line(&e, [-3.0, 11.101_802], [-3.0, 12.649_111]));
        // 齿顶弧（R=13，中心 90°）：76.657636° → 103.342364°（跨度 26.684728°）
        assert!(has_arc(&e, [0.0, 0.0], 13.0, 76.657_636, 103.342_364));
        // 齿根弧（R=11.5，槽心 0°）：−14.878335° → 14.878335°（跨度 29.756669°）
        assert!(has_arc(&e, [0.0, 0.0], 11.5, -14.878_335, 14.878_335));
        // 所有齿顶弧中心 = 90°+60k；所有齿根弧中心 = 0°+60k
        let mut tip_centers: Vec<String> = Vec::new();
        let mut root_centers: Vec<String> = Vec::new();
        for x in &e {
            if let EntityType::Arc(a) = x {
                let c = (a.start_angle.to_degrees() + a.end_angle.to_degrees()) / 2.0;
                let span = a.end_angle.to_degrees() - a.start_angle.to_degrees();
                if near(a.radius, 13.0) {
                    tip_centers.push(format!("{:.4}", c.rem_euclid(360.0)));
                    assert!(near(span, 26.684_728), "齿顶跨度 {span}");
                } else if near(a.radius, 11.5) {
                    root_centers.push(format!("{:.4}", c.rem_euclid(360.0)));
                    assert!(near(span, 29.756_669), "齿根跨度 {span}");
                }
            }
        }
        tip_centers.sort();
        root_centers.sort();
        let mut want_tip = vec![
            "90.0000", "150.0000", "210.0000", "270.0000", "330.0000", "30.0000",
        ];
        let mut want_root = vec![
            "0.0000", "60.0000", "120.0000", "180.0000", "240.0000", "300.0000",
        ];
        want_tip.sort();
        want_root.sort();
        assert_eq!(tip_centers, want_tip, "齿顶弧相位 90°+60k");
        assert_eq!(root_centers, want_root, "齿根弧相位 0°+60k（齿槽心线）");
        // 中心线 ±16（D+6 = 32）
        assert!(has_line(&e, [-16.0, 0.0], [16.0, 0.0]));
        assert!(has_line(&e, [0.0, -16.0], [0.0, 16.0]));
        // 用户定案：矩形花键端视**不画分度圆**（模板无；外花键才有）。
        assert!(
            !e.iter().any(|x| matches!(x, EntityType::Circle(_))),
            "矩形花键端视不画分度圆"
        );
    }

    /// 常规侧视图：矩形 L×D + 小径细线（2细线层）+ 中心线（L+6）。
    #[test]
    fn side_view_rectangle_and_layers() {
        let sp = RectSpline::from_code("6x23x26x6", None, 30.0).unwrap();
        let e = sp.side_view(30.0);
        assert_eq!(e.len(), 7);
        assert!(has_line(&e, [0.0, -13.0], [0.0, 13.0]));
        assert!(has_line(&e, [30.0, -13.0], [30.0, 13.0]));
        assert!(has_line(&e, [0.0, 13.0], [30.0, 13.0]));
        assert!(has_line(&e, [0.0, 11.5], [30.0, 11.5]));
        assert!(has_line(&e, [-3.0, 0.0], [33.0, 0.0]));
        let layer = |a: [f64; 2], b: [f64; 2]| {
            e.iter().find_map(|x| match x {
                EntityType::Line(l) if has_line(std::slice::from_ref(x), a, b) => {
                    Some(l.common.layer.as_str())
                }
                _ => None,
            })
        };
        assert_eq!(layer([0.0, 11.5], [30.0, 11.5]), Some(LAYER_THIN));
        assert_eq!(layer([0.0, 13.0], [30.0, 13.0]), Some(LAYER_MAIN));
        assert_eq!(layer([-3.0, 0.0], [33.0, 0.0]), Some(LAYER_CENTER));
    }

    /// 侧剖视图：小径线改 1轮廓实线层；HATCH = ANSI31 / 垂距 3.0mm（= 齿轮/花键统一口径）/ 2 环 × 4 LineEdge / flags 17。
    #[test]
    fn section_view_hatch_matches_template() {
        let sp = RectSpline::from_code("6x23x26x6", None, 30.0).unwrap();
        let e = sp.section_view(30.0);
        assert_eq!(e.len(), 8, "6 线 + 中心线 + 1 HATCH");
        let hatch = e
            .iter()
            .find_map(|x| match x {
                EntityType::Hatch(h) => Some(h),
                _ => None,
            })
            .expect("应有 HATCH");
        assert_eq!(hatch.common.layer, "5剖面线层");
        assert_eq!(hatch.pattern.name, "ANSI31");
        assert!((hatch.pattern_scale - crate::gear::HATCH_PATTERN_SCALE).abs() < 1e-12);
        // 垂距 = 3.0mm（宿主反旋转 offset 的 |dy|）。
        let ln = &hatch.pattern.lines[0];
        let dy = (-ln.offset.x * ln.angle.sin() + ln.offset.y * ln.angle.cos()).abs();
        assert!((dy - crate::gear::HATCH_SPACING_MM).abs() < 1e-9, "垂距 {dy} ≠ 3.0mm");
        assert_eq!(hatch.paths.len(), 2, "轴线上下两条带");
        for path in &hatch.paths {
            assert_eq!(path.edges.len(), 4, "每环 4 条 LineEdge");
            assert_eq!(path.flags.bits() & 0x11, 0x11, "flags=external|outermost");
            for edge in &path.edges {
                assert!(matches!(edge, BoundaryEdge::Line(_)));
            }
        }
        // 界线：上带 y 0→11.5、下带 0→−11.5，x 0→30（只填轴线↔小径，不填齿部）
        let mut ys: Vec<(f64, f64)> = Vec::new();
        for path in &hatch.paths {
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for edge in &path.edges {
                if let BoundaryEdge::Line(l) = edge {
                    lo = lo.min(l.start.y).min(l.end.y);
                    hi = hi.max(l.start.y).max(l.end.y);
                }
            }
            ys.push((lo, hi));
        }
        ys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        assert!(near(ys[0].0, -11.5) && near(ys[0].1, 0.0));
        assert!(near(ys[1].0, 0.0) && near(ys[1].1, 11.5));
        // 小径线落 1轮廓实线层（不是 2细线层）
        assert!(e.iter().any(|x| matches!(x, EntityType::Line(l)
            if near(l.start.y, 11.5) && near(l.end.y, 11.5) && l.common.layer == LAYER_MAIN)));
        assert!(!e.iter().any(|x| matches!(x, EntityType::Line(l)
            if near(l.start.y, 11.5) && l.common.layer == LAYER_THIN)));
    }

    /// 收尾弧口径：圆心 / 相切 / 终点 / l 公式 / 交角。
    #[test]
    fn runout_formula_and_angles() {
        let sp = RectSpline::from_code("6x23x26x6", None, 30.0).unwrap();
        let (h, r, rh) = (sp.depth(), sp.minor_radius(), sp.hob_radius());
        assert!(near(h, 1.5) && near(r, 11.5) && near(rh, 31.5));
        assert!(near(sp.runout(), 9.604_687), "l=√(h(2R−h)) = 9.6047");
        let a = sp.runout_end_angle();
        assert!((a - 72.247_210).abs() < 1e-4, "大径交点极角 {a}");
        // 交点 = 圆心 + R·(cos,a) 应为 (L+l, D/2)
        let c = [30.0, r + rh];
        let p = [
            c[0] + rh * a.to_radians().cos(),
            c[1] - rh * a.to_radians().sin(),
        ];
        assert!(near(p[0], 30.0 + sp.runout()) && near(p[1], sp.big / 2.0));
        // 与小径相切：切点 (L, r)
        let t = [
            c[0] + rh * 270.0_f64.to_radians().cos(),
            c[1] + rh * 270.0_f64.to_radians().sin(),
        ];
        assert!(near(t[0], 30.0) && near(t[1], r));
    }
}
