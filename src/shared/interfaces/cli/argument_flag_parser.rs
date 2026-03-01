pub fn optional_flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
}

pub fn required_flag(args: &[String], name: &str) -> Result<String, String> {
    optional_flag(args, name).ok_or_else(|| format!("Missing required flag: {name}"))
}
