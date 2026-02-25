use chrono::Local;
use reqwest::Client;
use serde::Serialize;

#[derive(Serialize)]
struct SmsPayload {
    trial_number: u32,
    p_id: i64,
    water_level: f32,
    rtt: u32,
    sent_at: String,
}

pub async fn sms_request(
    trial_number: u32,
    p_id: i64,
    water_level: f32,
    rtt: u32,
) -> Result<(), reqwest::Error> {
    let client = Client::new();
    let current_time = Local::now().format("%H:%M:%S%.3f").to_string();
    let payload = SmsPayload {
        trial_number,
        p_id,
        water_level,
        rtt,
        sent_at: current_time,
    };

    let response = client
        .post("http://127.0.0.1:3000/gateway")
        .json(&payload)
        .send()
        .await?;

    println!(
        "Request Sent | Trial: {} | Status: {}",
        trial_number,
        response.status()
    );
    Ok(())
}
