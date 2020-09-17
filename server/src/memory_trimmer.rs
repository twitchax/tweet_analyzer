extern "C" {
    #[allow(dead_code)]
    fn malloc_trim(__pad: usize) -> std::os::raw::c_int;
}

#[allow(dead_code)]
pub fn start() {
    let _ = tokio::task::spawn(async {
        loop {
            tokio::time::delay_for(tokio::time::Duration::from_secs(300)).await;

            // SAFETY: this binary is meant run in a linux container, and the bursty nature of
            // memory allocations means that footprint balloons.  Using the native `malloc_trim` to 
            // address this large footprint.
            unsafe { malloc_trim(20 * 1_024 * 1_024 /* 20 MB */) };
        }
    });
}