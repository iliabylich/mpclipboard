use tokio_tungstenite::tungstenite::handshake::server::{
    Callback, ErrorResponse, Request, Response,
};

pub(crate) struct Auth {
    name: String,
    token: String,
}

impl Auth {
    pub(crate) fn new(token: String) -> Self {
        Self {
            name: String::new(),
            token,
        }
    }

    pub(crate) fn into_name(self) -> String {
        self.name
    }
}

impl Callback for &mut Auth {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        let auth_err = |body: &str| -> ErrorResponse {
            Response::builder()
                .status(401)
                .body(Some(body.to_string()))
                .unwrap()
        };

        let name = request
            .headers()
            .get("Name")
            .ok_or_else(|| auth_err("no Name"))?
            .to_str()
            .map_err(|_| auth_err("no Name"))?;

        let auth_err = |body: &str| -> ErrorResponse {
            log::error!(target: name, "{body}");
            Response::builder()
                .status(401)
                .body(Some(body.to_string()))
                .unwrap()
        };

        log::info!(target: name, "Incoming {request:?}");

        let token = request
            .headers()
            .get("Token")
            .ok_or_else(|| auth_err("no Token"))?
            .to_str()
            .map_err(|_| auth_err("malformed Token"))?;

        if token != self.token {
            return Err(auth_err("invalid Token"));
        }

        self.name = name.to_string();

        log::info!(target: name, "auth passed");
        Ok(response)
    }
}
