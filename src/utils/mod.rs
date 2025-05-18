use std::time::Instant;

pub const INITIAL_FOV: f32 = 40_f32.to_radians();

pub fn timeit<F: Fn() -> T, T>(description: &str, f: F) -> T {
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    let minutes = duration.as_secs() / 60;
    let seconds = duration.as_secs() % 60;
    let milliseconds = duration.subsec_millis();
    println!(
        "{}: {}m {}s {}ms",
        description, minutes, seconds, milliseconds
    );
    result
}

pub fn timeit_mut<F: FnMut() -> T, T>(description: &str, mut f: F) -> T {
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    let minutes = duration.as_secs() / 60;
    let seconds = duration.as_secs() % 60;
    let milliseconds = duration.subsec_millis();
    println!(
        "{}: {}m {}s {}ms",
        description, minutes, seconds, milliseconds
    );
    result
}
