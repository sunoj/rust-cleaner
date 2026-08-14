use wd_40::config::Config;
fn main() {
    let toml = "max_age_days = 7\nartifact_types = [\"target\", \"node_modules\", \".next\", \"dist\", \"build\"]\n";
    let config: Config = toml::from_str(toml).unwrap();
    println!("{:?}", config);
}
