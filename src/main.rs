struct AddressSpace {
    data: [u8; 0xFFFF],
}

impl AddressSpace {

    fn new() -> AddressSpace {
        AddressSpace { data: [0; 0xFFFF] }
    }

    fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.data[addr as usize] = data;
    }

}

struct Instr<'a> {
    opcode: u8,
    data: Option<&'a [u8]>,
}

impl<'a> Instr<'a> {

    fn new(opcode: u8, data: Option<&[u8]>) -> Instr {
        Instr { opcode: opcode, data: data }
    }

    fn opcode(&self) -> u8 {
        self.opcode
    }

    fn param(&self, i: usize) -> u8 {
        self.data.expect("Instruction type does not carry parameters")[i]
    }

}

fn main() {
    println!("Hello, world!");
}
