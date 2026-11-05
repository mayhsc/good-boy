use std::{fs::File, io::Read};

use crate::{
    bus::{Bus, MemoryMap},
    cpu::Cpu,
};

fn load(name: &str) -> Cpu<MemoryMap> {
    let path = format!(
        "{}/tests/roms/{name}.gb",
        env!("CARGO_MANIFEST_DIR")
    );

    let mut rom = File::open(path).expect("Unable to open file");
    let mut buffer = Vec::new();
    rom.read_to_end(&mut buffer).unwrap();

    let mut bus = MemoryMap::new();
    bus.load(&buffer, None);

    Cpu::new(bus)
}

fn run_test(cpu: &mut Cpu<MemoryMap>, max_steps: u64) -> String {
    let mut output = String::new();

    for _ in 0..max_steps {
        cpu.step();

        if cpu.bus.read(0xFF02) == 0x81 {
            let byte = cpu.bus.read(0xFF01);
            output.push(byte as char);
            cpu.bus.write(0xFF02, 0x00);
        }

        if output.contains("Passed") || output.contains("Failed") {
            break;
        }
    }

    output
}

pub fn run_file(name: &str) {
    let mut cpu = load(name);
    let output = run_test(&mut cpu, 50_000_000);

    assert!(
        output.contains("Passed"),
        "{name} did not pass. Output: {output}"
    );
}
