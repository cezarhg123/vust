use std::sync::mpsc;

/// Blocks the current thread until vust has finished rendering the previous frame
/// 
/// Otherwise commands will keep queuing up in the vust command channel until the pc runs out of memory. This happens in cases where the main thread is only sending commands to the render thread,
/// therefore the render thread has more work to do than the main thread.
/// 
/// Always run VustSyncer.sync() right after Vust.reset_command_buffer()
pub struct VustSyncer {
    pub(super) allow_messages_recv: mpsc::Receiver<()>
}

impl VustSyncer {
    /// Always run right after Vust.reset_command_buffer()
    pub fn sync(&self) {
        // will block current thread
        self.allow_messages_recv.recv().unwrap();
    }
}
