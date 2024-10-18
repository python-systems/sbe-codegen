use std::mem::size_of;

// CHAR
pub const CHAR_SIZE: usize = size_of::<u8>();
pub const CHAR_NULL: &str = "u8::MIN";

// U8
pub const U8_SIZE: usize = size_of::<u8>();
pub const U8_NULL: &str = "u8::MAX";

// U16
pub const U16_SIZE: usize = size_of::<u16>();
pub const U16_NULL: &str = "u16::MAX";

// U32
pub const U32_SIZE: usize = size_of::<u32>();
pub const U32_NULL: &str = "u32::MAX";

// U64
pub const U64_SIZE: usize = size_of::<u64>();
pub const U64_NULL: &str = "u64::MAX";

// I8
pub const I8_SIZE: usize = size_of::<i8>();
pub const I8_NULL: &str = "i8::MIN";

// I16
pub const I16_SIZE: usize = size_of::<i16>();
pub const I16_NULL: &str = "i16::MIN";

// I32
pub const I32_SIZE: usize = size_of::<i32>();
pub const I32_NULL: &str = "i32::MIN";

// I64
pub const I64_SIZE: usize = size_of::<i64>();
pub const I64_NULL: &str = "i64::MIN";

// F32
pub const F32_SIZE: usize = size_of::<f32>();
pub const F32_NULL: &str = "f32::NAN";

// F64
pub const F64_SIZE: usize = size_of::<f64>();
pub const F64_NULL: &str = "f64::NAN";
