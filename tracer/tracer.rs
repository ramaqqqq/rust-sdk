use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use opentelemetry::sdk::{
    trace::{self, BatchSpanProcessor, TracerProvider, Tracer},
    Resource,
};
use opentelemetry::trace::TraceError;
use opentelemetry::exporter::trace::OtlpTraceExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_semantic_conventions as semcov;

pub async fn new_trace_provider(
    trace_endpoint: &str,
    resource: Resource,
) -> Result<TracerProvider, TraceError> {
    let exporter = OtlpTraceExporter::builder()
        .with_endpoint(trace_endpoint)
        .with_insecure() 
        .build()
        .await?;

    let batch_processor = BatchSpanProcessor::builder(exporter, tokio::spawn)
        .build();

    let tracer_provider = TracerProvider::builder()
        .with_span_processor(batch_processor)
        .with_resource(resource)
        .build();

    Ok(tracer_provider)
}

pub async fn init_tracing(
    service_name: &str,
    service_version: &str,
    endpoint: &str,
) -> Result<(Arc<Mutex<dyn Fn(context::Context) -> Result<(), Box<dyn Error>>>>, Tracer), Box<dyn Error>> {
    let resource = Resource::new(vec![
        semcov::resource::SERVICE_NAME.string(service_name.to_string()),
        semcov::resource::SERVICE_VERSION.string(service_version.to_string()),
    ]);

    let tracer_provider = new_trace_provider(endpoint, resource).await?;
    
    let tracer = tracer_provider.tracer("tracer");

    opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    let shutdown_func = Arc::new(Mutex::new(move |ctx| {
        Box::new(tracer_provider.shutdown().map_err(|e| Box::new(e) as Box<dyn Error>))
    }));

    Ok((shutdown_func, tracer))
}
