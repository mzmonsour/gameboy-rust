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

fn main() {
    println!("Hello, world!");
}
