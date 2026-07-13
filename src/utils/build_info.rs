pub fn get_build_time_utc() -> &'static str {
    let buildtime: &'static str = env!("BUILD_TIME");
    buildtime
}
