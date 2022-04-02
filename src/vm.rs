use std::io::{Cursor, Read};
use std::mem;

pub trait TransmuteConst {
    fn const_size() -> usize;
}

pub trait Transmute {
    fn size(&mut self) -> usize;
    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()>;
    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized;
}

macro_rules! _int_alloc_impl {
    ($($typ:ident),* $(,)*) => {
        $(
            impl TransmuteConst for $typ {
                fn const_size() -> usize {
                    mem::size_of::<$typ>()
                }
            }

            impl Transmute for $typ {
                fn size(&mut self) -> usize {
                    mem::size_of::<$typ>()
                }

                fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
                    let bytes: [u8; mem::size_of::<$typ>()] = self.to_be_bytes();
                    buf.extend_from_slice(&bytes);
                    Ok(())
                }

                fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self> {
                    let mut exact = [0u8; mem::size_of::<$typ>()];
                    buf.read_exact(&mut exact)?;
                    Ok(Self::from_be_bytes(exact))
                }
            }
        )*
    };
}

_int_alloc_impl! {
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128
}

impl TransmuteConst for f32 {
    fn const_size() -> usize {
        4
    }
}

impl TransmuteConst for bool {
    fn const_size() -> usize {
        1
    }
}

impl TransmuteConst for f64 {
    fn const_size() -> usize {
        8
    }
}

impl Transmute for bool {
    fn size(&mut self) -> usize {
        1
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        buf.extend_from_slice(&[if *self { 0x01 } else { 0x00 }]);
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut d = [0u8; 1];
        buf.read_exact(&mut d)?;
        Ok(d[0] == 0x01)
    }
}

impl Transmute for f32 {
    fn size(&mut self) -> usize {
        4
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        buf.extend_from_slice(&self.to_be_bytes());
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut exact = [0u8; 4];
        buf.read_exact(&mut exact)?;
        Ok(f32::from_be_bytes(exact))
    }
}

impl Transmute for f64 {
    fn size(&mut self) -> usize {
        8
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        buf.extend_from_slice(&self.to_be_bytes());
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut exact = [0u8; 8];
        buf.read_exact(&mut exact)?;
        Ok(f64::from_be_bytes(exact))
    }
}

impl Transmute for String {
    fn size(&mut self) -> usize {
        self.len() + 2
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        let bytes = self.as_bytes();
        let vec: [u8; 2] = unsafe { mem::transmute(self.len() as u16) };
        buf.extend_from_slice(&vec);
        buf.extend_from_slice(&bytes);
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self> {
        let len = u16::read(buf)?;
        let slice = &buf.to_owned().into_inner()[1..(len as usize) + 2];
        Ok(String::from_utf8(slice.to_vec())?)
    }
}

impl TransmuteConst for char {
    fn const_size() -> usize {
        4
    }
}

impl Transmute for char {
    fn size(&mut self) -> usize {
        4
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        let mut o = [0u8; 4];
        self.encode_utf8(&mut o);
        buf.extend_from_slice(&o);
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(String::from_utf8(buf.to_owned().into_inner())?
            .chars()
            .collect::<Vec<char>>()[0])
    }
}

impl<V> Transmute for Box<V>
where
    V: Transmute,
{
    fn size(&mut self) -> usize {
        V::size(self)
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        V::write(self, buf)
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Box::new(V::read(buf)?))
    }
}

impl<V> Transmute for Vec<V>
where
    V: Transmute,
{
    fn size(&mut self) -> usize {
        self.len() + 4
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        (self.len() as u32).write(buf)?;
        for ele in self {
            ele.write(buf)?;
        }
        Ok(())
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let len = u32::read(buf)? as usize;
        let mut vec = Vec::<V>::with_capacity(len);
        for _ in 0..len {
            vec.push(V::read(buf)?);
        }
        Ok(vec)
    }
}
