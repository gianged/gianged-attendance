fn main() {
    // Windows resource compilation for icon and manifest
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon_128.ico");
        res.compile().unwrap();
    }
}
