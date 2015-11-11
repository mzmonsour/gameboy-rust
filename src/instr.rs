use RegData;
use AddressSpace;

pub struct Instr {
    opcode: u8,
    data: Vec<u8>,
    cycles: u32,
}

impl Instr {

    pub fn parse(reg: &mut RegData, rom: &AddressSpace) -> Instr {
        let opcode = rom.read(reg.advance_pc());
        Instr {
            opcode: opcode,
            data: Vec::new(),
            cycles: 4,
        }
    }

    pub fn opcode(&self) -> u8 {
        self.opcode
    }

    pub fn cycles(&self) -> u32 {
        self.cycles
    }

    pub fn param(&self, i: usize) -> u8 {
        self.data[i]
    }

    pub fn param_u16(&self, i: usize) -> u16 {
        (self.data[i] as u16) | ((self.data[i + 1] as u16) << 8)
    }

}
