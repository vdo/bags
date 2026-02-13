use crate::types::{AlertDirection, NotificationMethod};

pub fn send_alert(
    method: NotificationMethod,
    ntfy_topic: &str,
    coin_name: &str,
    target: f64,
    current: f64,
    direction: AlertDirection,
) {
    let dir_str = match direction {
        AlertDirection::Above => "above",
        AlertDirection::Below => "below",
    };
    let title = format!("bags: {} alert", coin_name);
    let body = format!(
        "{} hit {} target {:.2} (now {:.2})",
        coin_name, dir_str, target, current
    );

    match method {
        NotificationMethod::None => {}
        NotificationMethod::Desktop => {
            send_desktop(&title, &body);
        }
        NotificationMethod::Ntfy => {
            if !ntfy_topic.is_empty() {
                send_ntfy(ntfy_topic, &title, &body);
            }
        }
        NotificationMethod::Both => {
            send_desktop(&title, &body);
            if !ntfy_topic.is_empty() {
                send_ntfy(ntfy_topic, &title, &body);
            }
        }
    }
}

fn send_desktop(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show();
}

fn send_ntfy(topic: &str, title: &str, body: &str) {
    let url = format!("https://ntfy.sh/{}", topic);
    let title = title.to_string();
    let body = body.to_string();
    // Fire-and-forget in a background task
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client
            .post(&url)
            .header("Title", title)
            .body(body)
            .send()
            .await;
    });
}
