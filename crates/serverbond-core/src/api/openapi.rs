//! The route index is also the source for the machine-readable HTTP contract.
use serde_json::{json, Map, Value};

fn expand(path: &str) -> Vec<String> {
    let mut offset = 0;
    while let Some(start) = path[offset..].find('{').map(|n| n + offset) {
        let Some(end) = path[start..].find('}').map(|n| n + start) else {
            break;
        };
        let choices = &path[start + 1..end];
        if choices.contains('|') {
            return choices
                .split('|')
                .flat_map(|choice| {
                    expand(&format!("{}{choice}{}", &path[..start], &path[end + 1..]))
                })
                .collect();
        }
        offset = end + 1;
    }
    vec![path.into()]
}

fn fields(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

fn body_schema(method: &str, path: &str) -> Option<Value> {
    let string = json!({"type":"string"});
    let boolean = json!({"type":"boolean"});
    Some(match (method, path) {
        ("PUT", "/settings") | ("POST", "/settings/validate") => {
            json!({"type":"object", "description":"Settings: GET /settings çıktısı. Doğrulama aynı Manager kurallarını kullanır.", "example":crate::model::Settings::default()})
        }
        ("POST", "/projects") => fields(json!({"name":string,"path":string}), &["name", "path"]),
        ("POST", "/projects/create") => {
            fields(json!({"name":string,"parent":string}), &["name", "parent"])
        }
        ("POST", "/projects/import") => fields(
            json!({"url":string,"name":string,"branch":string}),
            &["url"],
        ),
        ("POST", "/projects/import-folders") => {
            fields(json!({"paths":{"type":"array","items":string}}), &["paths"])
        }
        ("POST", "/projects/{id}/php" | "/projects/{id}/php/repair") => {
            fields(json!({"version":string}), &["version"])
        }
        ("PUT", "/projects/{id}/env") => fields(json!({"content":string}), &["content"]),
        ("PUT", "/projects/{id}/jobs") => fields(
            json!({"workers":{"type":"array","items":{"type":"object"}},"schedule":{"type":"object"}}),
            &["workers", "schedule"],
        ),
        ("PUT", "/projects/{id}/release") => {
            json!({"type":"object","description":"ProjectRelease: GET /projects/{id}/release çıktısı"})
        }
        ("POST", "/projects/{id}/failed-jobs/retry") => {
            fields(json!({"job":{"type":["string","null"]}}), &[])
        }
        ("POST", "/projects/{id}/database/restore") => fields(json!({"path":string}), &["path"]),
        ("POST", "/mysql/password" | "/postgres/password") => fields(
            json!({"password":{"type":"string","writeOnly":true}}),
            &["password"],
        ),
        ("POST", "/github/token" | "/tunnel/token" | "/tunnel/apply") => fields(
            json!({"token":{"type":"string","writeOnly":true}}),
            &["token"],
        ),
        ("POST", "/tunnel/auto-start") => fields(json!({"autoStart":boolean}), &["autoStart"]),
        ("POST", "/permissions/grant") => fields(json!({"defender":boolean}), &["defender"]),
        ("PUT", "/desktop/appearance") => fields(
            json!({"theme":{"type":"string","enum":["system","light","dark"]}}),
            &["theme"],
        ),
        ("PUT", "/desktop") => fields(
            json!({"preferences":{"type":"object","properties":{"closeToTray":boolean,"startMinimized":boolean}},"autostart":boolean}),
            &["preferences", "autostart"],
        ),
        ("POST", "/desktop/navigate") => fields(
            json!({"page":{"type":"string","enum":["overview","packages","projects","logs","services","settings","updates"]}}),
            &["page"],
        ),
        ("POST", "/updates/install") => fields(
            json!({"version":string,"confirm":{"type":"boolean","const":true}}),
            &["version", "confirm"],
        ),
        _ => return None,
    })
}

pub fn document() -> Value {
    let mut paths = Map::new();
    for (method, full_path, description) in super::routes() {
        let (path, query) = full_path.split_once('?').unwrap_or((full_path, ""));
        let relative = path.strip_prefix("/api/v1").unwrap_or(path);
        for path in expand(if relative.is_empty() { "/" } else { relative }) {
            let mut parameters = Vec::new();
            for segment in path.split('/') {
                if let Some(name) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                    parameters.push(
                        json!({"name":name,"in":"path","required":true,"schema":{"type":"string"}}),
                    );
                }
            }
            if !query.is_empty() {
                parameters.push(json!({"name":"source","in":"query","required":false,"schema":{"type":"string","default":"php"},"description":"php, schedule veya worker:<workerId>"}));
            }
            let mut operation = json!({
                "summary":description,
                "operationId":format!("{}_{}",method.to_lowercase(),path.replace(['/', '{', '}', '.', '-'],"_")),
                "parameters":parameters,
                "responses":{
                    "200":{"description":"İşlem tamamlandı","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Success"}}}},
                    "400":{"$ref":"#/components/responses/Error"},
                    "401":{"$ref":"#/components/responses/Error"},
                    "404":{"$ref":"#/components/responses/Error"},
                    "413":{"$ref":"#/components/responses/Error"},
                    "500":{"$ref":"#/components/responses/Error"},
                    "501":{"$ref":"#/components/responses/Error"},
                    "503":{"$ref":"#/components/responses/Error"}
                }
            });
            if path == "/health" {
                operation["security"] = json!([]);
            }
            if path == "/openapi.json" {
                operation["responses"]["200"] = json!({"description":"Doğrudan OpenAPI 3.1 belgesi; yanıt zarfı kullanılmaz","content":{"application/json":{"schema":{"type":"object","required":["openapi","info","paths"]}}}});
            }
            if matches!(
                path.as_str(),
                "/desktop/exit" | "/desktop/restart" | "/updates/install"
            ) {
                operation["responses"]["202"] = json!({"description":"Yanıt gönderildi; işlem uygulanacak. Bağlantı kapanabilir.","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Success"}}}});
            }
            if let Some(schema) = body_schema(method, &path) {
                operation["requestBody"] = json!({"required": !path.ends_with("/failed-jobs/retry"),"content":{"application/json":{"schema":schema}}});
            }
            paths.entry(path).or_insert_with(|| json!({}))[method.to_lowercase()] = operation;
        }
    }
    json!({
        "openapi":"3.1.0",
        "info":{"title":"ServerBond Management API","version":env!("CARGO_PKG_VERSION"),"description":"Yerel yönetim API'si. Masaüstü uçları için capabilities.desktop=true gerekir. Settings, işçi ve release nesnelerinin tam değerlerini ilgili GET uçlarından alın."},
        "servers":[{"url":"/api/v1"}],
        "security":[{"bearerAuth":[]}],
        "paths":paths,
        "components":{
            "securitySchemes":{"bearerAuth":{"type":"http","scheme":"bearer"}},
            "schemas":{
                "Success":{"type":"object","required":["ok","data"],"properties":{"ok":{"const":true},"data":{}}},
                "Error":{"type":"object","required":["ok","error"],"properties":{"ok":{"const":false},"error":{"type":"string"}}}
            },
            "responses":{"Error":{"description":"Doğrulama, kimlik doğrulama, desteklenmeyen host veya işlem hatası","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Error"}}}}}
        }
    })
}
