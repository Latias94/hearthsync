use super::event::TaskProgressEvent;

pub trait TaskProgressSink {
    fn push(&mut self, event: TaskProgressEvent);

    fn task_id(&self) -> Option<&str> {
        None
    }
}

#[derive(Debug, Default)]
pub struct NoopProgressSink;

impl TaskProgressSink for NoopProgressSink {
    fn push(&mut self, _event: TaskProgressEvent) {}
}

#[derive(Debug, Default)]
pub struct VecTaskProgressSink {
    task_id: Option<String>,
    events: Vec<TaskProgressEvent>,
}

impl VecTaskProgressSink {
    pub fn with_task_id(task_id: impl Into<String>) -> Self {
        Self {
            task_id: Some(task_id.into()),
            events: Vec::new(),
        }
    }

    pub fn events(&self) -> &[TaskProgressEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<TaskProgressEvent> {
        self.events
    }
}

impl TaskProgressSink for VecTaskProgressSink {
    fn push(&mut self, mut event: TaskProgressEvent) {
        attach_task_id_if_missing(&mut event, self.task_id());
        self.events.push(event);
    }

    fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }
}

pub struct CallbackProgressSink<F> {
    task_id: Option<String>,
    on_progress: F,
}

impl<F> CallbackProgressSink<F> {
    pub fn new(on_progress: F) -> Self {
        Self {
            task_id: None,
            on_progress,
        }
    }

    pub fn with_task_id(task_id: impl Into<String>, on_progress: F) -> Self {
        Self {
            task_id: Some(task_id.into()),
            on_progress,
        }
    }
}

impl<F> TaskProgressSink for CallbackProgressSink<F>
where
    F: FnMut(TaskProgressEvent),
{
    fn push(&mut self, mut event: TaskProgressEvent) {
        attach_task_id_if_missing(&mut event, self.task_id());
        (self.on_progress)(event);
    }

    fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }
}

fn attach_task_id_if_missing(event: &mut TaskProgressEvent, task_id: Option<&str>) {
    if event.task_id.is_none() {
        event.task_id = task_id.map(ToOwned::to_owned);
    }
}
