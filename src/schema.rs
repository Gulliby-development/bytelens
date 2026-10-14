/// The 14 integer types of a schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntType {
    U8,
    I8,
    U16Le,
    U16Be,
    I16Le,
    I16Be,
    U32Le,
    U32Be,
    I32Le,
    I32Be,
    U64Le,
    U64Be,
    I64Le,
    I64Be,
}

impl IntType {
    pub const ALL: [IntType; 14] = [
        IntType::U8,
        IntType::I8,
        IntType::U16Le,
        IntType::U16Be,
        IntType::I16Le,
        IntType::I16Be,
        IntType::U32Le,
        IntType::U32Be,
        IntType::I32Le,
        IntType::I32Be,
        IntType::U64Le,
        IntType::U64Be,
        IntType::I64Le,
        IntType::I64Be,
    ];

    /// Size in bytes.
    pub fn size(self) -> usize {
        match self {
            IntType::U8 | IntType::I8 => 1,
            IntType::U16Le | IntType::U16Be | IntType::I16Le | IntType::I16Be => 2,
            IntType::U32Le | IntType::U32Be | IntType::I32Le | IntType::I32Be => 4,
            IntType::U64Le | IntType::U64Be | IntType::I64Le | IntType::I64Be => 8,
        }
    }

    pub fn signed(self) -> bool {
        matches!(
            self,
            IntType::I8
                | IntType::I16Le
                | IntType::I16Be
                | IntType::I32Le
                | IntType::I32Be
                | IntType::I64Le
                | IntType::I64Be
        )
    }

    pub fn big_endian(self) -> bool {
        matches!(
            self,
            IntType::U16Be
                | IntType::I16Be
                | IntType::U32Be
                | IntType::I32Be
                | IntType::U64Be
                | IntType::I64Be
        )
    }

    /// The type name as written in a schema.
    pub fn name(self) -> &'static str {
        match self {
            IntType::U8 => "u8",
            IntType::I8 => "i8",
            IntType::U16Le => "u16le",
            IntType::U16Be => "u16be",
            IntType::I16Le => "i16le",
            IntType::I16Be => "i16be",
            IntType::U32Le => "u32le",
            IntType::U32Be => "u32be",
            IntType::I32Le => "i32le",
            IntType::I32Be => "i32be",
            IntType::U64Le => "u64le",
            IntType::U64Be => "u64be",
            IntType::I64Le => "i64le",
            IntType::I64Be => "i64be",
        }
    }

    pub fn from_name(name: &str) -> Option<IntType> {
        IntType::ALL.into_iter().find(|t| t.name() == name)
    }

    /// Smallest and largest value of the type.
    pub fn range(self) -> (i128, i128) {
        let bits = self.size() as u32 * 8;
        if self.signed() {
            (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
        } else {
            (0, (1i128 << bits) - 1)
        }
    }
}

/// The base of a field type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    Int(IntType),
    Bytes,
    Str,
    /// Index into `Schema::structs`.
    Struct(usize),
}

/// The COUNT of an array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Count {
    Fixed(u32),
    Field(String),
    Rest,
}

/// A literal a field must equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Int(i128),
    Text(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub line: usize,
    pub type_text: String,
    pub base: Base,
    pub count: Option<Count>,
    pub literal: Option<Literal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDef {
    pub name: String,
    pub line: usize,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schema {
    pub root: Vec<Field>,
    pub structs: Vec<StructDef>,
}
