use tauri::Manager;
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use std::net::TcpListener;

const AUTH_PORT: u16 = 17291;

fn generate_state() -> String {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).expect("failed to generate random state");
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}

#[tauri::command]
fn open_in_browser(url: String) {
    if url.starts_with("https://") || url.starts_with("http://") {
        let _ = open::that(&url);
    }
}

#[tauri::command]
fn get_csrf_state(state: tauri::State<'_, Arc<Mutex<String>>>) -> String {
    state.lock().unwrap().clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let expected_state = Arc::new(Mutex::new(generate_state()));
    let managed_state = expected_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(managed_state)
        .invoke_handler(tauri::generate_handler![open_in_browser, get_csrf_state])
        .setup(move |app| {
            let handle = Arc::new(app.handle().clone());

            // Start local HTTP server to receive OAuth callback
            let handle_clone = handle.clone();
            let state_clone = expected_state.clone();
            std::thread::spawn(move || {
                let listener = match TcpListener::bind(format!("127.0.0.1:{}", AUTH_PORT)) {
                    Ok(l) => l,
                    Err(e) => { eprintln!("Auth server bind error: {}", e); return; }
                };

                for stream in listener.incoming() {
                    if let Ok(mut stream) = stream {
                        let mut buf = vec![0u8; 8192];
                        let n = stream.read(&mut buf).unwrap_or(0);
                        let request = String::from_utf8_lossy(&buf[..n]);

                        if request.starts_with("GET /callback") {
                            let path = request.lines().next().unwrap_or("");
                            let query_start = path.find('?').unwrap_or(path.len());
                            let query = &path[query_start..].trim_end_matches(" HTTP/1.1").trim_end_matches(" HTTP/1.0");

                            let mut token = String::new();
                            let mut email = String::new();
                            let mut name = String::new();
                            let mut state = String::new();

                            for pair in query.trim_start_matches('?').split('&') {
                                let mut kv = pair.splitn(2, '=');
                                let k = kv.next().unwrap_or("");
                                let v = urlencoding::decode(kv.next().unwrap_or("")).unwrap_or_default().to_string();
                                match k {
                                    "token" => token = v,
                                    "email" => email = v,
                                    "name" => name = v,
                                    "state" => state = v,
                                    _ => {}
                                }
                            }

                            // Validate CSRF state (always required)
                            if state.is_empty() {
                                let html = "<html><body style='font-family:system-ui;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;background:#0f0f0f;color:#ef4444'><div style='text-align:center'><h2>Authentication Error</h2><p style='color:#78716c'>Missing security state. Please try again from the app.</p></div></body></html>";
                                let response = format!("HTTP/1.1 403 Forbidden\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}", html);
                                let _ = stream.write_all(response.as_bytes());
                                continue;
                            }
                            {
                                let valid_state = {
                                    let expected = state_clone.lock().unwrap();
                                    state == *expected
                                };
                                if !valid_state {
                                    let html = "<html><body style='font-family:system-ui;display:flex;align-items:center;justify-content:center;height:100vh;margin:0;background:#0f0f0f;color:#ef4444'><div style='text-align:center'><h2>Authentication Error</h2><p style='color:#78716c'>Invalid or expired session. Please try again from the app.</p></div></body></html>";
                                    let response = format!("HTTP/1.1 403 Forbidden\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}", html);
                                    let _ = stream.write_all(response.as_bytes());
                                    continue;
                                }
                                // Rotate state after successful use (one-time use)
                                let mut expected = state_clone.lock().unwrap();
                                *expected = generate_state();
                            }

                            let html = "<html><head><style>*{margin:0;padding:0;box-sizing:border-box}body{font-family:-apple-system,BlinkMacSystemFont,system-ui,sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;background:#0f0f0f;color:#f5f5f4}.card{text-align:center;background:#1a1a1a;padding:48px 40px;border-radius:20px;border:1px solid rgba(255,255,255,0.08);max-width:380px}.logo{width:64px;height:64px;margin:0 auto 16px;background:linear-gradient(135deg,#6366f1,#06b6d4);border-radius:16px;display:flex;align-items:center;justify-content:center;font-size:32px}h2{font-size:22px;font-weight:700;margin-bottom:8px;background:linear-gradient(135deg,#6366f1,#06b6d4);-webkit-background-clip:text;-webkit-text-fill-color:transparent}p{color:#78716c;font-size:14px;line-height:1.5}.countdown{margin-top:16px;font-size:12px;color:#57534e}</style></head><body><div class='card'><div class='logo'>&#10003;</div><h2>Connected to Runik AI</h2><p>You can close this window.<br>Returning to the app...</p><p class='countdown' id='cd'>Closing in 3s</p></div><script>let s=3;const t=setInterval(()=>{s--;document.getElementById('cd').textContent='Closing in '+s+'s';if(s<=0){clearInterval(t);window.close();document.getElementById('cd').textContent='You can close this tab.';}},1000);</script></body></html>";
                            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}", html);
                            let _ = stream.write_all(response.as_bytes());

                            if !token.is_empty() {
                                // Navigate using fragment (#) instead of query (?) so token doesn't appear in server logs
                                let nav_url = format!("https://runikapp.com/app/#auth_token={}&auth_email={}&auth_name={}",
                                    urlencoding::encode(&token), urlencoding::encode(&email), urlencoding::encode(&name));
                                if let Some(window) = handle_clone.get_webview_window("main") {
                                    let _ = window.navigate(nav_url.parse().unwrap());
                                }
                            }
                        } else {
                            let response = "HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n";
                            let _ = stream.write_all(response.as_bytes());
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
