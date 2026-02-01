use zond_common::{config::Config, models::range::IpCollection};

pub fn scan(_ips: IpCollection, _cfg: &Config) -> anyhow::Result<()> {
    anyhow::bail!("'scan' subcommand not implemented yet");
}
