use axum::{routing::post, Json, Router};
use chrono::{Local, NaiveTime};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Deserialize)]
struct SmsRequest {
    trial_number: u32,
    p_id: i64,
    water_level: f32,
    rtt: u32,
    sent_at: String,
}

#[derive(Serialize)]
struct SmsResponse {
    message: String,
    delay_ms: i64,
}

async fn handle_sms(Json(payload): Json<SmsRequest>) -> Json<SmsResponse> {
    let now = Local::now();
    let sent_time = NaiveTime::parse_from_str(&payload.sent_at, "%H:%M:%S%.3f")
        .unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap());

    let duration = now.time() - sent_time;
    let delay_ms = duration.num_milliseconds();
    let status = match payload.water_level {
        w if w >= 115.0 => "NOT PASSABLE TO ALL VEHICLES (Chest Deep)",
        w if w >= 94.0 => "NOT PASSABLE TO ALL VEHICLES (Waist Deep)",
        w if w >= 66.0 => "NOT PASSABLE TO ALL VEHICLES (Tire Deep)",
        w if w >= 48.0 => "NOT PASSABLE TO LIGHT VEHICLES (Knee Deep)",
        w if w >= 33.0 => "NOT PASSABLE TO LIGHT VEHICLES (Half Tire)",
        w if w >= 25.0 => "PASSABLE (Half Knee Deep)",
        w if w >= 20.0 => "PASSABLE (Gutter Deep)",
        _ => "NORMAL (No Flooding)",
    };

    let alert_msg = format!(
        "[{}] STATUS: {} | Water: {}m | System Delay: {}ms",
        now.format("%H:%M:%S.%3f"),
        status,
        payload.water_level,
        delay_ms
    );

    println!("--- SHEM PROJECT ---");
    println!("{}", alert_msg);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("sms_log.csv")
    {
        let estimated_sendtime = (payload.rtt as i64 / 2) + delay_ms;

        let log_line = format!(
            "{},{},{},{},{},{}\n",
            now.format("%Y-%m-%d %H:%M:%S.%3f"),
            payload.p_id,
            payload.trial_number,
            estimated_sendtime,
            payload.water_level,
            delay_ms
        );

        let _ = file.write_all(log_line.as_bytes());
    }

    Json(SmsResponse {
        message: alert_msg,
        delay_ms,
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/gateway", post(handle_sms));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("SMS Gateway Simulator active on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}
