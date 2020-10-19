use anyhow::Context;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

/// API LINK.
// ➜  ~ substrate-api-sidecar
// SAS:
//   📦 LOG:
//      ✅ LEVEL: "info"
//      ✅ JSON: false
//      ✅ FILTER_RPC: false
//      ✅ STRIP_ANSI: false
//   📦 SUBSTRATE:
//      ✅ WS_URL: "ws://127.0.0.1:9944"
//   📦 EXPRESS:
//      ✅ BIND_HOST: "127.0.0.1"
//      ✅ PORT: 8080
// 2020-10-19 19:04:21 info: Connected to chain Development on the parity-p//olkadot client at ws://127.0.0.1:9944
// 2020-10-19 19:04:21 info: Listening on http://127.0.0.1:8080/

const SIDECAR_API: &'static str = "http://127.0.0.1:8080";

#[derive(Debug, StructOpt)]
#[structopt(name = "polkadot_payout_reader", about = "Reads the payout.")]
struct Args {
    #[structopt(short, long)]
    accountId: String,

    #[structopt(short, long)]
    depth: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct StackingPayoutResp {
    at: serde_json::Value,
    erasPayouts: serde_json::Value,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Welcome to Polkadot pending payout reader ");

    let response = reqwest::get(SIDECAR_API)
        .await
        .with_context(|| "failed to connect sidecar api endpoint")?;

    if response.status().as_u16() != 200 {
        println!(
            "Unable to connect sidecart api. Please recheck your instance: {}",
            SIDECAR_API
        );

        return Ok(());
    }

    // Documentation url: https://paritytech.github.io/substrate-api-sidecar/dist/
    let network_url = format!("{}/{}", SIDECAR_API, "node/version");
    let networks = reqwest::get(&network_url)
        .await
        .with_context(|| "failed to connect sidecar node/version endpoint")?;

    let json: serde_json::Value = networks.json().await?;

    let chain = &json["chain"];

    if let serde_json::Value::String(ss) = chain {
        if ss == "None" {
            println!("The chain is broken !!!.");
            return Ok(());
        }
    };

    // Should be Devopment if you are running using --dev flag.
    println!("The current chain is {}", chain);

    // Now we get payout.

    let args = Args::from_args();
    println!("Your Input: {:?}", args);

    let network_url = format!(
        "{}/accounts/{}/staking-payouts?depth={depth}&unclaimedOnly=true",
        SIDECAR_API,
        args.accountId,
        depth = args.depth + 1
    );

    println!(
        "Making api request to get staking information {}",
        network_url
    );

    let stacking_resp = reqwest::get(&network_url)
        .await
        .with_context(|| "Error: unable to query stacking info")?;

    // Rules of three
    if stacking_resp.status().as_u16() != 200 {
        println!(
            "Unable to query stacking info. Please recheck the instance: {} {:?}",
            network_url, stacking_resp
        );

        return Ok(());
    }

    let stacking_text_resp = stacking_resp.text().await?;

    //dbg!(&stacking_text_resp);

    let mut total_payout: f64 = 0.;

    let stacking_resp: StackingPayoutResp =
        serde_json::from_str(&stacking_text_resp).with_context(|| "Invalid response from api")?;

    if let serde_json::Value::Array(json_payouts) = stacking_resp.erasPayouts {
        if json_payouts.len() == 0 {
            println!("There is 0 payout");
        }
        for payout_value in json_payouts {
            let payouts = payout_value["payouts"].as_array();

            if payouts.is_none() {
                println!("Api didn't gave payouts");
                return Ok(());
            }

            let payouts = payouts.unwrap();
            if payouts.len() == 0 {
                println!("There was 0 payout");
                return Ok(());
            }

            for payout in payouts {
                let payout = payout.as_object().unwrap();

                let nominator_staking_payout = payout.get("nominatorStakingPayout");
                let claimes = payout.get("claimed");

                if nominator_staking_payout.is_some() && claimes.is_some() {
                    let nominator_staking_payout =
                        nominator_staking_payout.unwrap().as_f64().unwrap();
                    let claimes = claimes.unwrap().as_bool().unwrap();

                    if claimes == false {
                        total_payout += nominator_staking_payout
                    }
                }
            }
        }
    } else {
        println!("There is no payouts for given depth");
    }

    println!("Total Payout, {}", total_payout);

    // End
    Ok(())
}
