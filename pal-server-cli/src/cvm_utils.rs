use std::time::Duration;

use itertools::Itertools;
use tencent_cloud_sdk::{
    client::{TencentCloudClient, cvm::cvm_instance::{InstanceState, Price}},
    constant::{InstanceType, Region},
};
use tokio::time::{Instant, sleep};

use crate::server_status::ServiceInstanceType;

/// return cheapest (price, (region, zone, instance_type))
pub async fn query_spot_paid_price(
    client: &TencentCloudClient,
    candidate_regions: &[Region],
    instance_type: &ServiceInstanceType,
) -> anyhow::Result<Vec<(Price, (Region, String, InstanceType))>> {
    let candidate_instance_type = instance_type.to_list();

    let mut handles = vec![];

    for region in candidate_regions {
        let zones = client.cvm().zone().describe_zone(region).await?;
        if let Some(zones) = zones {
            for (zone, instance_type) in zones.iter().cartesian_product(candidate_instance_type.iter()) {
                let client = client.clone();
                let zone = zone.clone();
                let region = region.clone();
                let instance_type = instance_type.clone();
                tokio::time::sleep(Duration::from_millis(100)).await;
                handles.push(tokio::spawn(async move {
                    (
                        client
                            .cvm()
                            .instances()
                            .query_price(&region, &zone, &instance_type)
                            .await,
                        region,
                        zone,
                        instance_type,
                    )
                }));
            }
        }
    }

    let mut price_result = vec![];
    for handle in handles {
        if let (Ok(price), region, zone, instance_type) = handle.await? {
            price_result.push((price, (region, zone, instance_type)));
        }
    }
    price_result.sort_by(|a, b| {
        a.0.instance_price
            .unit_price_discount
            .total_cmp(&b.0.instance_price.unit_price_discount)
    });
    println!("All spot price results: {:#?}", price_result);
    Ok(price_result)
    // price_result.into_iter().next().ok_or(anyhow::anyhow!(
    //     "failed to get any available instance of {candidate_instance_type:?}"
    // ))
}

pub async fn query_cvm_ip(client: &TencentCloudClient, region: &Region, instance_id: &str) -> anyhow::Result<String> {
    let timeout_duration = Duration::from_secs(62);
    let start_time = Instant::now();

    loop {
        // 进行轮询查询的操作
        // ...
        let resp = client.cvm().instances().describe_instance(region).await?;

        if let Some(instance) = resp
            .response
            .instance_set
            .into_iter()
            .find(|i| i.instance_id == instance_id && i.instance_state == InstanceState::RUNNING)
        {
            break instance
                .public_ip_addresses
                .ok_or(anyhow::anyhow!(format!("running cvm without ip?")))?
                .into_iter()
                .next()
                .ok_or(anyhow::anyhow!(format!("running cvm without ip?")));
        }

        // 检查是否超时
        if Instant::now() - start_time >= timeout_duration {
            break Err(anyhow::anyhow!("query cvm create status and ip timeout"));
        }

        // 等待一段时间再进行下一次轮询
        sleep(Duration::from_secs(5)).await;
    }
}
