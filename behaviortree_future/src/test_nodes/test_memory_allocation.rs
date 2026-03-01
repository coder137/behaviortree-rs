#[cfg(feature = "test-dhat")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

pub struct DhatTester {
    #[cfg(feature = "test-dhat")]
    name: &'static str,
    #[cfg(feature = "test-dhat")]
    profiler: Option<dhat::Profiler>,
}

impl DhatTester {
    pub fn new(_name: &'static str) -> Self {
        #[cfg(not(feature = "test-dhat"))]
        let this = Self {};

        #[cfg(feature = "test-dhat")]
        let this = Self::new_dhat(_name);

        this
    }

    #[cfg(feature = "test-dhat")]
    fn new_dhat(name: &'static str) -> Self {
        const DHAT_TEST_DIR: &str = "target/dhat";
        if !std::fs::exists(DHAT_TEST_DIR).unwrap() {
            let _ignore = std::fs::create_dir_all(DHAT_TEST_DIR);
        }
        let profiler = dhat::Profiler::builder()
            .file_name(format!("{DHAT_TEST_DIR}/{name}.json"))
            .build();
        Self {
            name,
            profiler: Some(profiler),
        }
    }
}

#[cfg(feature = "test-dhat")]
impl Drop for DhatTester {
    fn drop(&mut self) {
        let stats = dhat::HeapStats::get();
        println!("-------------");
        println!("{}:\n{stats:?}", self.name);
        drop(self.profiler.take().unwrap());
        println!("-------------");
    }
}
