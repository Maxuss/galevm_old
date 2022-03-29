use crate::vm::AllocSized;
use anyhow::bail;
use std::io::Cursor;

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum BinaryOp {
    Assign, // =, unused by default
    Add,    // +
    Sub,    // -
    Div,    // /
    Mul,    // *
    Mod,    //  %
    And,    // &&
    Or,     // ||
    Eq,     // ==
    Neq,    // !=
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    BitRsh, // >>
    BitLsh, // <<
}

impl AllocSized for BinaryOp {
    fn size(&mut self) -> usize {
        1
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            BinaryOp::Assign => 0x00u8,
            BinaryOp::Add => 0x01,
            BinaryOp::Sub => 0x02,
            BinaryOp::Div => 0x03,
            BinaryOp::Mul => 0x04,
            BinaryOp::Mod => 0x05,
            BinaryOp::And => 0x06,
            BinaryOp::Or => 0x07,
            BinaryOp::Eq => 0x08,
            BinaryOp::Neq => 0x09,
            BinaryOp::BitAnd => 0x0A,
            BinaryOp::BitOr => 0x0B,
            BinaryOp::BitXor => 0x0C,
            BinaryOp::BitRsh => 0x0D,
            BinaryOp::BitLsh => 0x0E,
        }
        .write(buf)
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(match u8::read(buf)? {
            0x00 => BinaryOp::Assign,
            0x01 => BinaryOp::Add,
            0x02 => BinaryOp::Sub,
            0x03 => BinaryOp::Div,
            0x04 => BinaryOp::Mul,
            0x05 => BinaryOp::Mod,
            0x06 => BinaryOp::And,
            0x07 => BinaryOp::Or,
            0x08 => BinaryOp::Eq,
            0x09 => BinaryOp::Neq,
            0x0A => BinaryOp::BitAnd,
            0x0B => BinaryOp::BitOr,
            0x0C => BinaryOp::BitXor,
            0x0D => BinaryOp::BitRsh,
            0x0E => BinaryOp::BitLsh,
            _ => bail!("Invalid binary operator provided!"),
        })
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub enum UnaryOp {
    Neg, // !
    Rev, // ~
}

impl AllocSized for UnaryOp {
    fn size(&mut self) -> usize {
        1
    }

    fn write(&mut self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        match self {
            UnaryOp::Neg => 0x00u8,
            UnaryOp::Rev => 0x01,
        }
        .write(buf)
    }

    fn read(buf: &mut Cursor<Vec<u8>>) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(match u8::read(buf)? {
            0x00 => UnaryOp::Neg,
            0x01 => UnaryOp::Rev,
            _ => bail!("Invalid unary operator provided!"),
        })
    }
}
