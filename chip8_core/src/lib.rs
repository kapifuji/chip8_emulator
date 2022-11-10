use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

struct Ram {
    memory: [u8; 0xFFF],
}

struct Rom {
    pub data: Vec<u8>,
}

#[derive(Default, Debug)]
struct Cpu {
    v: [u8; 16],                 // 8bit general register
    i: u16,                      // register for store memory address
    delay_timer: Arc<Mutex<u8>>, // delay register
    sound_timer: Arc<Mutex<u8>>, // sound timer register
    sp: u8,                      // stack pointer
    stack: [u16; 16],            // stack
    pc: u16,                     // program counter
    is_start: Arc<AtomicBool>,   // is running CPU
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            sp: 0x0f,
            pc: 0x200,
            ..Cpu::default()
        }
    }

    pub fn run(&self) {
        self.is_start.store(true, Ordering::Relaxed);

        let delay = self.delay_timer.clone();
        let sound = self.sound_timer.clone();
        let is_start = self.is_start.clone();

        thread::spawn(move || loop {
            let start = Instant::now();

            if is_start.load(Ordering::Relaxed) == false {
                break;
            }

            let mut delay_t = delay.lock().unwrap();
            if *delay_t > 0 {
                *delay_t -= 1;
            }

            // 音はどうやって鳴らすか...
            let mut sound_t = sound.lock().unwrap();
            if *sound_t > 0 {
                *sound_t -= 1;
                println!("sound is enable...");
            }

            thread::sleep(Duration::new(1, 0).div_f64(60.0) - start.elapsed());
        });
    }

    pub fn stop(&mut self) {
        self.is_start.store(false, Ordering::Relaxed);
    }

    pub fn tick(&mut self, ram: &mut Ram, key: Option<Key>, display: &mut Display) {
        // fetch opcode
        let op1 = ram.fetch_by_address(self.pc) >> 4;
        let op2 = ram.fetch_by_address(self.pc) & 0x0f;
        let op3 = ram.fetch_by_address(self.pc + 1) >> 4;
        let op4 = ram.fetch_by_address(self.pc + 1) & 0x0f;

        self.pc += 2;

        // decode and excute
        match (op1, op2, op3, op4) {
            (0x0, 0x0, 0xe, 0x0) => {
                // Clear display
                display.clear();
            }
            (0x0, 0x0, 0xe, 0xe) => {
                // Return Subroutine
                if self.sp == 0x0f {
                    panic!("Not found return address: 0x00EE")
                }
                self.sp += 1;
                self.pc = self.stack[self.sp as usize];
            }
            (0x1, n1, n2, n3) => {
                // Branch Program to 0xNNN
                let dest_address = ((n1 as u16) << 8) + ((n2 as u16) << 4) + (n3 as u16);
                self.pc = dest_address;
            }
            (0x2, n1, n2, n3) => {
                // Call Subroutine from 0xNNN
                let dest_address = ((n1 as u16) << 8) + ((n2 as u16) << 4) + (n3 as u16);
                self.stack[self.sp as usize] = self.pc;
                if self.sp == 0x00 {
                    panic!("Stack Overflow: 0x2NNN")
                }
                self.pc = dest_address;
                self.sp -= 1;
            }
            (0x3, x, n1, n2) => {
                // Skip instruction if Vx == 0xNN
                let cmp_num = (n1 << 4) + n2;
                if self.v[x as usize] == cmp_num {
                    self.pc += 2;
                }
            }
            (0x4, x, n1, n2) => {
                // Skip instruction if Vx != 0xNN
                let cmp_num = (n1 << 4) + n2;
                if self.v[x as usize] != cmp_num {
                    self.pc += 2;
                }
            }
            (0x5, x, y, 0x0) => {
                // Skip instruction if Vx == Vy
                if self.v[x as usize] == self.v[y as usize] {
                    self.pc += 2;
                }
            }
            (0x6, x, n1, n2) => {
                // Vx == 0xNN
                let num = (n1 << 4) + n2;
                self.v[x as usize] = num;
            }
            (0x7, x, n1, n2) => {
                // Vx += 0xNN
                let vx = self.v[x as usize];
                let num = (n1 << 4) + n2;

                self.v[x as usize] = if (vx as u16 + num as u16) > 0xff {
                    self.v[0xf] = 1;
                    (vx as u16 + num as u16 - 0xff - 1) as u8
                } else {
                    self.v[0xf] = 0;
                    vx + num
                };
            }
            (0x8, x, y, 0x0) => {
                // Vx = Vy
                self.v[x as usize] = self.v[y as usize];
            }
            (0x8, x, y, 0x1) => {
                // Vx |= Vy
                self.v[x as usize] |= self.v[y as usize];
            }
            (0x8, x, y, 0x2) => {
                // Vx &= Vy
                self.v[x as usize] &= self.v[y as usize];
            }
            (0x8, x, y, 0x3) => {
                // Vx ^= Vy
                self.v[x as usize] ^= self.v[y as usize];
            }
            (0x8, x, y, 0x4) => {
                // Vx += Vy, Vf = 1 if result is overflow else Vf = 0
                let vx = self.v[x as usize];
                let vy = self.v[y as usize];

                self.v[x as usize] = if (vx as u16 + vy as u16) as u16 > 0xff {
                    self.v[0xf] = 1;
                    (vx as u16 + vy as u16 - 0xff - 1) as u8
                } else {
                    self.v[0xf] = 0;
                    vx + vy
                }
            }
            (0x8, x, y, 0x5) => {
                // Vx -= Vy, Vf = 0 if result is underflow else Vf = 1
                let vx = self.v[x as usize];
                let vy = self.v[y as usize];

                self.v[x as usize] = if vx < vy {
                    self.v[0xf] = 0;
                    (vx as u16 + 0xff + 1 - vy as u16) as u8
                } else {
                    self.v[0xf] = 1;
                    vx - vy
                }
            }
            (0x8, x, _y, 0x6) => {
                // Vx >>= 1, before set lowest bit of Vx to Vf
                self.v[0xf] = self.v[x as usize] & 0x01;
                self.v[x as usize] >>= 1;
            }
            (0x8, x, y, 0x7) => {
                // Vx = Vy - Vx, Vf = 0 if result is underflow else Vf = 1
                let vx = self.v[x as usize];
                let vy = self.v[y as usize];

                self.v[x as usize] = if vy < vx {
                    self.v[0xf] = 0;
                    (vy as u16 + 0xff + 1 - vx as u16) as u8
                } else {
                    self.v[0xf] = 1;
                    vy - vx
                }
            }
            (0x8, x, _y, 0xe) => {
                // Vx <<= 1, before set highest bit of Vx to Vf
                self.v[0xf] = self.v[x as usize] & 0x80;
                self.v[x as usize] <<= 1;
            }
            (0x9, x, y, 0x0) => {
                // Skip instruction if Vx != Vy
                if self.v[x as usize] != self.v[y as usize] {
                    self.pc += 2;
                }
            }
            (0xa, n1, n2, n3) => {
                // i = 0xNNN
                let num = ((n1 as u16) << 8) + ((n2 as u16) << 4) + (n3 as u16);
                self.i = num;
            }
            (0xb, n1, n2, n3) => {
                // pc = V0 + 0xNNN
                let num = ((n1 as u16) << 8) + ((n2 as u16) << 4) + (n3 as u16);
                self.pc = (self.v[0x0] as u16) + num;
            }
            (0xc, x, n1, n2) => {
                // Vx = [random value] & 0xNN
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let rand: u8 = rng.gen();

                let num = (n1 << 4) + n2;
                self.v[x as usize] = rand & num;
            }
            (0xd, x, y, n) => {
                let vx = self.v[x as usize];
                let vy = self.v[y as usize];

                let mut is_fliped = false;
                for i in 0..n {
                    is_fliped = display.write(vx, vy + i, ram.memory[(self.i + i as u16) as usize]);
                }
                self.v[0x0f] = if is_fliped { 1 } else { 0 };
            }
            (0xe, x, 0x9, 0xe) => {
                if let Some(key) = key {
                    if key as u8 == self.v[x as usize] {
                        self.pc += 2;
                    }
                }
            }
            (0xe, x, 0xa, 0x1) => {
                if let Some(key) = key {
                    if key as u8 != self.v[x as usize] {
                        self.pc += 2;
                    }
                } else {
                    self.pc += 2;
                }
            }
            (0xf, x, 0x0, 0x7) => {
                self.v[x as usize] = *self.delay_timer.lock().unwrap();
            }
            (0xf, x, 0x0, 0xa) => {
                if let Some(key) = key {
                    self.v[x as usize] = key as u8;
                } else {
                    self.pc -= 2;
                }
            }
            (0xf, x, 0x1, 0x5) => {
                *self.delay_timer.lock().unwrap() = self.v[x as usize];
            }
            (0xf, x, 0x1, 0x8) => {
                *self.sound_timer.lock().unwrap() = self.v[x as usize];
            }
            (0xf, x, 0x1, 0xe) => {
                self.i += self.v[x as usize] as u16;
            }
            (0xf, x, 0x2, 0x9) => {
                let offset = (self.v[x as usize] as u16) * 5;
                self.i = 0x100 + offset;
            }
            (0xf, x, 0x3, 0x3) => {
                let num = self.v[x as usize];
                ram.memory[self.i as usize] = num / 100;
                ram.memory[(self.i + 1 as u16) as usize] = (num % 100) / 10;
                ram.memory[(self.i + 2 as u16) as usize] = num % 10;
            }
            (0xf, x, 0x5, 0x5) => {
                for i in 0..x + 1 {
                    ram.memory[(self.i + i as u16) as usize] = self.v[i as usize];
                }
            }
            (0xf, x, 0x6, 0x5) => {
                for i in 0..x + 1 {
                    self.v[i as usize] = ram.memory[(self.i + i as u16) as usize];
                }
            }
            _ => {
                panic!("Illigal opecode")
            }
        }
    }
}

impl Rom {
    pub fn new(program: Vec<u8>) -> Self {
        Rom { data: program }
    }
}

impl Ram {
    pub fn new() -> Self {
        Ram { memory: [0; 0xFFF] }
    }

    pub fn fetch_by_address(&self, address: u16) -> u8 {
        if address > 0xFFF {
            panic!("Fetch Out of memory");
        }
        self.memory[address as usize]
    }

    pub fn load_rom(&mut self, rom: &Rom) {
        self.load_fontset();
        let rom_offset = 0x200;
        self.memory[rom_offset..rom_offset + rom.data.len()].copy_from_slice(&rom.data)
    }

    fn load_fontset(&mut self) {
        let fontset = vec![
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];
        self.memory[0x100..0x100 + fontset.len()].copy_from_slice(&fontset);
    }
}

// 1 2 3 C
// 4 5 6 D
// 7 8 9 E
// A 0 B F
#[derive(Clone, Copy)]
pub enum Key {
    ZERO,
    ONE,
    TWO,
    THREE,
    FOUR,
    FIVE,
    SIX,
    SEVEN,
    EIGHT,
    NINE,
    A,
    B,
    C,
    D,
    E,
    F,
}

struct Display {
    data: [[bool; 64]; 32],
}

impl Display {
    pub fn new() -> Self {
        Display {
            data: [[false; 64]; 32],
        }
    }

    pub fn clear(&mut self) {
        self.data = [[false; 64]; 32];
    }

    pub fn write(&mut self, x: u8, y: u8, data: u8) -> bool {
        let mut is_fliped = false;
        for i in 0..8 {
            let before = self.data[y as usize][(x + i) as usize];
            self.data[y as usize][(x + i) as usize] ^= ((data >> (7 - i)) & 0x01) == 1;
            if (self.data[y as usize][(x + i) as usize] == false) && (before == true) {
                is_fliped = true
            }
        }
        is_fliped
    }
}

pub struct Chip8Core {
    rom: Rom,
    ram: Ram,
    cpu: Cpu,
    display: Display,
}

impl Chip8Core {
    pub fn new(rom_bytes: Vec<u8>) -> Self {
        let rom = Rom::new(rom_bytes);
        let mut ram = Ram::new();
        ram.load_rom(&rom);

        Chip8Core {
            rom: rom,
            ram: ram,
            cpu: Cpu::new(),
            display: Display::new(),
        }
    }

    pub fn tick(&mut self, key: Option<Key>) {
        self.cpu.tick(&mut self.ram, key, &mut self.display);
    }

    pub fn run(&self) {
        self.cpu.run();
    }

    pub fn stop(&mut self) {
        self.cpu.stop();
    }

    pub fn get_display_data(&self) -> [[bool; 64]; 32] {
        self.display.data
    }

    pub fn out_log(&self) {
        println!("{:?}", self.cpu);
    }
}
