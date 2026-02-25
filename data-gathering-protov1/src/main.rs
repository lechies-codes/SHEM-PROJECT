mod gateway_sim;
use chrono::Local;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;
use std::{thread, time::Duration as StdDuration};

#[tokio::main]
async fn main() {
    let port_name = "/dev/ttyUSB0";
    let baud_rate = 115200;

    let port = loop {
        match serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(1000))
            .open()
        {
            Ok(p) => break p,
            Err(e) => {
                eprintln!("Failed to open {}: {}. Retrying in 2s...", port_name, e);
                thread::sleep(StdDuration::from_secs(2));
            }
        }
    };

    let reader = BufReader::new(port);
    let mut trial_number: u32 = 1;
    let total_trials: u32 = 3;
    let mut block_count: u32 = 0;
    let mut sum_latency: u32 = 0;
    let mut latencies: Vec<u32> = Vec::with_capacity(100);
    let mut start_p_id: Option<i64> = None;
    let mut data_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(format!("data{}.csv", trial_number))
        .expect("Failed to open data file");
    writeln!(data_file, "Date&Time,Packet ID,Water Level,RTT").unwrap();
    data_file.flush().unwrap();

    for line in reader.lines() {
        if trial_number > total_trials {
            break;
        }

        if let Ok(raw_line) = line {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() || trimmed.contains("packetid") {
                continue;
            }

            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(data_file, "{},{}", timestamp, trimmed).unwrap();
            data_file.flush().unwrap();

            let parts: Vec<&str> = trimmed.split(',').collect();
            if parts.len() >= 3 {
                let Ok(p_id) = parts[0].parse::<i64>() else {
                    continue;
                };
                let Ok(water_level) = parts[1].parse::<f32>() else {
                    continue;
                };
                let Ok(rtt) = parts[2].parse::<u32>() else {
                    continue;
                };

                if start_p_id.is_none() {
                    start_p_id = Some(p_id);
                }

                block_count += 1;
                sum_latency += rtt;
                latencies.push(rtt);

                if block_count % 25 == 0 {
                    tokio::spawn(async move {
                        if let Err(e) =
                            gateway_sim::sms_request(trial_number, p_id, water_level, rtt).await
                        {
                            eprintln!("SMS simulator request failed: {}", e);
                        }
                    });
                }

                if block_count >= 100 {
                    let avg_latency = sum_latency as f32 / 100.0;
                    let mut jitter_sum = 0.0;
                    for i in 1..latencies.len() {
                        jitter_sum += (latencies[i] as f32 - latencies[i - 1] as f32).abs();
                    }
                    let jitter = jitter_sum / (latencies.len() - 1) as f32;
                    let expected = (p_id - start_p_id.unwrap() + 1) as f32;
                    let pdr = (block_count as f32 / expected) * 100.0;
                    let mut summary_file = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(format!("T{}_Summary.txt", trial_number))
                        .expect("Failed to open summary file");

                    writeln!(summary_file, "--- TRIAL #{} SUMMARY ---", trial_number).unwrap();
                    writeln!(summary_file, "Avg Latency: {:.2} ms", avg_latency).unwrap();
                    writeln!(summary_file, "Jitter: {:.2} ms", jitter).unwrap();
                    writeln!(summary_file, "PDR: {:.2} %", pdr).unwrap();
                    writeln!(
                        summary_file,
                        "Packet Range: {} to {}",
                        start_p_id.unwrap(),
                        p_id
                    )
                    .unwrap();
                    summary_file.flush().unwrap();

                    println!("Trial {} complete", trial_number);

                    block_count = 0;
                    sum_latency = 0;
                    latencies.clear();
                    start_p_id = None;
                    trial_number += 1;

                    if trial_number <= total_trials {
                        data_file = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .truncate(true)
                            .open(format!("data{}.csv", trial_number))
                            .expect("Failed to open data file");
                        writeln!(data_file, "Date&Time,Packet ID,Water Level,RTT").unwrap();
                        data_file.flush().unwrap();
                    }
                }
            }
        }
    }
}
