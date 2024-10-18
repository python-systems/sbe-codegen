use anyhow::Result;

use crate::generator::rust::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::ByteOrder;
use genco::prelude::*;

impl ModuleGenerator<'_> {
    pub fn generate_encoder_traits(&self) -> Result<()> {
        let byte_order_conversion = match self.schema.byte_order {
            ByteOrder::BigEndian => "to_be_bytes",
            ByteOrder::LittleEndian => "to_le_bytes",
        };

        let encoder_tokens: Tokens<Rust> = quote! {
            use crate::error::*;

            #[derive(Debug, Default)]
            pub struct WriteBuf<'a> {
                data: &'a mut [u8],
            }

            impl<'a> WriteBuf<'a> {
                pub fn new(data: &mut [u8]) -> WriteBuf {
                    WriteBuf { data }
                }

                #[inline]
                pub fn put_bytes_at(&mut self, index: usize, bytes: &[u8]) -> Result<()> {
                    let data_end = index + bytes.len();

                    if data_end > self.data.len() {
                        return Err(SbeError::CodecOutOfBounds(data_end, self.data.len()));
                    }

                    self.data[index..data_end].copy_from_slice(bytes);

                    Ok(())
                }

                #[inline]
                pub fn put_u8_at(&mut self, index: usize, value: u8) -> Result<()> {
                    self.put_bytes_at(index, &u8::$byte_order_conversion(value))
                }

                #[inline]
                pub fn put_i8_at(&mut self, index: usize, value: i8) -> Result<()> {
                    self.put_bytes_at(index, &i8::$byte_order_conversion(value))
                }

                #[inline]
                pub fn put_i16_at(&mut self, index: usize, value: i16) -> Result<()> {
                    self.put_bytes_at(index, &i16::$byte_order_conversion(value))
                }

                #[inline]
                pub fn put_i32_at(&mut self, index: usize, value: i32) -> Result<()> {
                    self.put_bytes_at(index, &i32::$byte_order_conversion(value))
                }

                #[inline]
                pub fn put_i64_at(&mut self, index: usize, value: i64) -> Result<()> {
                    self.put_bytes_at(index, &i64::$byte_order_conversion(value))
                }

                #[inline]
                pub fn put_u16_at(&mut self, index: usize, value: u16) -> Result<()> {
                    self.put_bytes_at(index, &u16::$byte_order_conversion(value))
                }

                #[inline]
                pub fn put_u32_at(&mut self, index: usize, value: u32) -> Result<()> {
                    self.put_bytes_at(index, &u32::$byte_order_conversion(value))
                }

                #[inline]
                pub fn put_u64_at(&mut self, index: usize, value: u64) -> Result<()> {
                    self.put_bytes_at(index, &u64::$byte_order_conversion(value))
                }

                #[inline]
                pub fn put_f32_at(&mut self, index: usize, value: f32) -> Result<()> {
                    self.put_bytes_at(index, &f32::$byte_order_conversion(value))
                }

                #[inline]
                pub fn put_f64_at(&mut self, index: usize, value: f64) -> Result<()> {
                    self.put_bytes_at(index, &f64::$byte_order_conversion(value))
                }

                #[inline]
                pub fn split_at_mut(&mut self, index: usize) -> Result<(WriteBuf, WriteBuf)> {
                    // Self cannot be used in this method, as it inherits generics
                    // (and most importantly, lifetimes).
                    // Let self have lifetime 'a. Then the WriteBuf being returned has to have
                    // lifetime 'b, where 'b <= 'a. If we used self, WriteBuf would be
                    // implicitly WriteBuf<'a>, which would not work.
                    if index >= self.data.len() {
                        return Err(SbeError::CodecOutOfBounds(index, self.data.len()));
                    }

                    let (before, after) = self.data.split_at_mut(index);
                    Ok((Self::new(before), Self::new(after)))
                }

                #[inline]
                pub fn split_at_mut_owned(self, index: usize) -> Result<(Self, Self)> {
                    if index >= self.data.len() {
                        return Err(SbeError::CodecOutOfBounds(index, self.data.len()));
                    }

                    let (before, after) = self.data.split_at_mut(index);
                    Ok((Self::new(before), Self::new(after)))
                }
            }
        };

        write_file(&self.path.join("encoder.rs"), &self.config, encoder_tokens)?;

        Ok(())
    }
}
