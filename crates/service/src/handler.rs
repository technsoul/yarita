use yarita_application::FocusApp;
use yarita_protocol::{Request, Response};

/// Maps one wire request onto a use case.
pub fn handle(app: &mut FocusApp, request: Request) -> Response {
    let result = match request {
        Request::Snapshot => Ok(()),
        Request::Start => app.start(),
        Request::Pause => app.pause(),
        Request::Resume => app.resume(),
        Request::Stop => app.stop(),
        Request::Skip => app.skip(),
        Request::SetStrict(on) => app.set_strict(on),
        Request::UpdateConfig(config) => app.update_config(config),
        Request::AddSite(site) => app.add_site(&site),
        Request::RemoveSite(site) => app.remove_site(&site),
        Request::AddApp(name) => app.add_app(&name),
        Request::RemoveApp(name) => app.remove_app(&name),
    };
    let snapshot = app.snapshot();
    match result {
        Ok(()) => Response::Ok(snapshot),
        Err(e) => Response::Refused { reason: e.to_string(), snapshot },
    }
}
