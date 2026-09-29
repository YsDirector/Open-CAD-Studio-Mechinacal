use serde::{Deserialize, Serialize};

/// Stable identifier for a type in the embedded registry.
///
/// `TypeId` serializes as a plain string so it can be used as a JSON object
/// key. The inner value is intentionally private; consumers use `as_str()`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TypeId(String);

impl TypeId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Classification of a type in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeKind {
    Struct,
    Enum,
    Newtype,
    Tuple,
    Unit,
}

/// Description of one field in a struct or enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub type_id: TypeId,
    pub optional: bool,
    pub is_sequence: bool,
}

/// Description of one enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariantInfo {
    pub name: String,
    pub discriminant: u32,
    pub fields: Vec<FieldInfo>,
}

/// Description of one method parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_id: TypeId,
}

/// Description of one method on a type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<TypeId>,
}

/// Full description of one type in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: TypeId,
    pub kind: TypeKind,
    pub fields: Vec<FieldInfo>,
    pub variants: Vec<EnumVariantInfo>,
    pub methods: Vec<MethodInfo>,
    pub doc: Option<String>,
}

/// Collection of all traced types, keyed by their `TypeId`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeRegistry {
    pub types: std::collections::BTreeMap<TypeId, TypeInfo>,
}
