use std::ffi::OsString;
use std::sync::mpsc;
use std::time::Duration;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlHandlerResult, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerEvents},
    service_dispatcher,
};

const SERVICE_NAME: &str = "AgentCollectTool";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, system_service_main);

pub fn run_service() -> Result<(), windows_service::Error> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

fn system_service_main(_arguments: Vec<OsString>) {
    if let Err(_e) = run_app() {
        // Handle error, maybe log it
    }
}

fn run_app() -> Result<(), anyhow::Error> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                let _ = shutdown_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: windows_service::service::ServiceState::Running,
        controls_accepted: windows_service::service::ServiceControlAccept::STOP,
        exit_code: 0,
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    // Main loop
    loop {
        // Check for shutdown signal
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        // TODO: Perform agent tasks here
        std::thread::sleep(Duration::from_secs(1));
    }

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: windows_service::service::ServiceState::Stopped,
        controls_accepted: windows_service::service::ServiceControlAccept::empty(),
        exit_code: 0,
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}
