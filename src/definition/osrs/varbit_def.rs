use super::Definition;
use crate::extension::ReadExt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::io;
use std::io::BufReader;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct VarbitDefinition {
    pub id: u16,
    pub varp_id: u16,
    pub least_significant_bit: u8,
    pub most_significant_bit: u8,
}

impl Definition for VarbitDefinition {
    fn new(id: u16, buffer: &[u8]) -> crate::Result<Self> {
        let mut reader = BufReader::new(buffer);
        let varbit_def = decode_buffer(id, &mut reader)?;

        Ok(varbit_def)
    }
}

fn decode_buffer(id: u16, reader: &mut BufReader<&[u8]>) -> io::Result<VarbitDefinition> {
    let mut varbit_def = VarbitDefinition {
        id,
        varp_id: 0,
        least_significant_bit: 0,
        most_significant_bit: 0,
    };

    let opcode = reader.read_u8()?;

    if opcode == 1 {
        varbit_def.varp_id = reader.read_u16()?;
        varbit_def.least_significant_bit = reader.read_u8()?;
        varbit_def.most_significant_bit = reader.read_u8()?;
    }

    Ok(varbit_def)
}
