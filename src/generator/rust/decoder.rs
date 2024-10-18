use anyhow::Result;

use crate::generator::rust::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::ByteOrder;
use genco::prelude::*;

impl ModuleGenerator<'_> {
    pub fn generate_decoder_traits(&self) -> Result<()> {
        let byte_order_conversion = match self.schema.byte_order {
            ByteOrder::BigEndian => "from_be_bytes",
            ByteOrder::LittleEndian => "from_le_bytes",
        };

        let decoder_tokens: Tokens<Rust> = quote! {
            use std::convert::TryInto;
            use crate::error::*;

            #[derive(Debug, Default)]
            pub struct ReadBuf<'a> {
                data: &'a [u8],
            }

            impl<'a> ReadBuf<'a> {
                #[inline]
                pub fn new(data: &'a [u8]) -> Self {
                    Self { data }
                }

                #[inline]
                fn get_bytes<const COUNT: usize>(slice: &[u8]) -> Result<[u8; COUNT]> {
                    slice.try_into().map_err(|_| {
                        SbeError::WrongSliceSize(format!(
                            "expected {} bytes, got {} bytes",
                            COUNT,
                            slice.len()
                        ))
                    })
                }

                #[inline]
                fn get_bytes_at<const COUNT: usize>(&self, index: usize) -> Result<[u8; COUNT]> {
                    let data_end = index + COUNT;

                    if data_end > self.data.len() {
                        return Err(SbeError::CodecOutOfBounds(data_end, self.data.len()));
                    }

                    Self::get_bytes(&self.data[index..data_end])
                }

                #[inline]
                pub fn get_slice_at(&self, index: usize, len: usize) -> Result<&[u8]> {
                    let data_end = index + len;

                    if data_end > self.data.len() {
                        return Err(SbeError::CodecOutOfBounds(data_end, self.data.len()));
                    }

                    Ok(&self.data[index..data_end])
                }

                #[inline]
                pub fn get_u8_at(&self, index: usize) -> Result<u8> {
                    Ok(u8::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn get_u16_at(&self, index: usize) -> Result<u16> {
                    Ok(u16::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn get_u32_at(&self, index: usize) -> Result<u32> {
                    Ok(u32::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn get_u64_at(&self, index: usize) -> Result<u64> {
                    Ok(u64::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn get_i8_at(&self, index: usize) -> Result<i8> {
                    Ok(i8::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn get_i16_at(&self, index: usize) -> Result<i16> {
                    Ok(i16::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn get_i32_at(&self, index: usize) -> Result<i32> {
                    Ok(i32::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn get_i64_at(&self, index: usize) -> Result<i64> {
                    Ok(i64::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn get_f32_at(&self, index: usize) -> Result<f32> {
                    Ok(f32::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn get_f64_at(&self, index: usize) -> Result<f64> {
                    Ok(f64::$byte_order_conversion(self.get_bytes_at(index)?))
                }

                #[inline]
                pub fn split_at(&self, index: usize) -> Result<(Self, Self)> {
                    if index >= self.data.len() {
                        return Err(SbeError::CodecOutOfBounds(index, self.data.len()));
                    }

                    let (left, right) = self.data.split_at(index);
                    Ok((Self::new(left), Self::new(right)))
                }
            }
        };

        write_file(&self.path.join("decoder.rs"), &self.config, decoder_tokens)?;

        Ok(())
    }
}
