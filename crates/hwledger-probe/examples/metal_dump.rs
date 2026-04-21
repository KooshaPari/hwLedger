#[cfg(target_os = "macos")]
fn main() {
    use hwledger_probe::GpuProbe;
    let probe = hwledger_probe::MetalProbe::new().expect("init");
    println!("enumerate: {:?}", probe.enumerate());
    println!("utilization: {:?}", probe.utilization(0));
    println!("temperature: {:?}", probe.temperature(0));
    println!("power_draw: {:?}", probe.power_draw(0));
    println!("free_vram: {:?}", probe.free_vram(0));
}
#[cfg(not(target_os = "macos"))]
fn main() {}
