use super::Definition;
use crate::extension::ReadExt;
#[cfg(feature = "serde-derive")]
use serde::{Deserialize, Serialize};
use std::io;
use std::io::BufReader;

#[derive(Clone, Eq, PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde-derive", derive(Serialize, Deserialize))]
pub struct InventoryDefinition {
    pub id: u16,
    pub capacity: Option<u16>,
}

impl Definition for InventoryDefinition {
    fn new(id: u16, buffer: &[u8]) -> crate::Result<Self> {
        let mut reader = BufReader::new(buffer);
        let item_def = decode_buffer(id, &mut reader)?;

        Ok(item_def)
    }
}

fn decode_buffer(id: u16, reader: &mut BufReader<&[u8]>) -> io::Result<InventoryDefinition> {
    let mut inv_def = InventoryDefinition { id, capacity: None };

    loop {
        let opcode = reader.read_u8()?;
        match opcode {
            0 => break,
            2 => inv_def.capacity = reader.read_u16().ok(),
            _ => {}
        }
    }

    Ok(inv_def)
}
