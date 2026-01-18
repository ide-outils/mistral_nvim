use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use nvim_oxi::api;

use crate::nvim::model;

pub mod buffer_modifier;
pub mod chat;

pub use buffer_modifier::BufferModifierGroupedUndo;
pub use chat::{Chat, ChatForm, ChatState};

pub trait Locker {
    type Locked;

    fn inner(&self) -> &Arc<Mutex<Self::Locked>>;
    #[track_caller]
    fn lock<'lock>(&'lock self) -> std::sync::MutexGuard<'lock, Self::Locked> {
        let mut attempts = 0;
        loop {
            match self.inner().try_lock() {
                Ok(lock) => return lock,
                Err(err) => match err {
                    std::sync::TryLockError::Poisoned(poison_error) => {
                        todo!("handle poisoned state (reload from file ?). {poison_error}")
                    }
                    std::sync::TryLockError::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        if attempts == 44 {
                            crate::log_libuv!(Off, "Thread was blocked here.");
                            // TODO: softly kill by unwindind in main.
                            panic!("Thread locked.");
                        }
                    }
                },
            }
            attempts += 1;
        }
    }
    fn ptr_eq(&self, other: &impl Locker<Locked = Self::Locked>) -> bool {
        Arc::ptr_eq(self.inner(), other.inner())
    }
}
pub struct SharedState(Arc<Mutex<State>>);
impl Locker for SharedState {
    type Locked = State;

    fn inner(&self) -> &Arc<Mutex<Self::Locked>> {
        &self.0
    }
}
impl SharedState {
    pub fn clone(to_clone: &Self) -> Self {
        Self(Arc::clone(&to_clone.0))
    }
}
pub type SenderMistral = tokio::sync::mpsc::UnboundedSender<crate::messages::NvimEnveloppe>;

#[derive(Default)]
pub struct Chats {
    paths: HashMap<PathBuf, Chat>,
    buffers: HashMap<api::Buffer, Chat>,
}
impl Chats {
    pub fn insert(&mut self, chat: Chat) -> Chat {
        let path = { chat.lock().path.clone() };
        if let Some(chat_found) = self.paths.get(&path) {
            if chat.ptr_eq(chat_found) {
                chat
            } else {
                Chat::clone(&chat)
            }
        } else {
            let buffer = chat.lock().buffer.clone();
            self.paths.insert(path, Chat::clone(&chat));
            self.buffers.insert(buffer, Chat::clone(&chat));
            chat
        }
    }
    pub fn get_by_buffer(&mut self, id: &api::Buffer) -> Option<&Chat> {
        self.buffers.get(&id)
    }
    pub fn get_by_path(&mut self, path: &PathBuf) -> Option<&Chat> {
        self.paths.get(path)
    }
}

pub struct State {
    pub buffer_modifiers: HashMap<api::Buffer, BufferModifierGroupedUndo>,
    pub tx_mistral: tokio::sync::mpsc::UnboundedSender<crate::messages::NvimEnveloppe>,
    pub chats: Chats,
    pub fim: HashMap<api::Buffer, usize>,
}

impl State {
    pub fn new(tx_mistral: SenderMistral) -> SharedState {
        SharedState(Arc::new(Mutex::new(State {
            tx_mistral,
            // bufffers: Default::default(),
            buffer_modifiers: Default::default(),
            chats: Default::default(),
            fim: Default::default(),
        })))
    }
    pub fn add_fim(&mut self, buffer: &api::Buffer) -> usize {
        let id = self.fim.entry(buffer.clone()).or_insert(0);
        *id += id.saturating_add(1);
        id.clone()
    }
    pub fn remove_fim(&mut self, buffer: &api::Buffer) {
        let Some(id) = self.fim.get_mut(buffer) else { return };
        *id = id.saturating_sub(1);
        if *id == 0 {
            self.fim.remove(buffer);
        }
    }

    #[track_caller]
    pub fn start_insertion_successive(
        &mut self,
        buffer: &api::Buffer,
        id: usize,
        cursor: model::Cursor,
    ) -> crate::Result<()> {
        if let Some(buffer_modifier) = self.buffer_modifiers.get_mut(&buffer) {
            buffer_modifier.start_insertion_successive(id, cursor)?;
        } else {
            let mut bm = BufferModifierGroupedUndo::new(buffer)?;
            bm.start_insertion_successive(id, cursor)?;
            self.buffer_modifiers.insert(buffer.clone(), bm);
        }
        Ok(())
    }
    #[track_caller]
    pub fn start_replace_line(
        &mut self,
        buffer: &api::Buffer,
        id: usize,
        row_final: model::Row,
        length_initial: usize,
    ) -> crate::Result<()> {
        if let Some(buffer_modifier) = self.buffer_modifiers.get_mut(&buffer) {
            buffer_modifier.start_replacement_line(id, row_final, length_initial)?;
        } else {
            let mut bm = BufferModifierGroupedUndo::new(buffer)?;
            bm.start_replacement_line(id, row_final, length_initial)?;
            self.buffer_modifiers.insert(buffer.clone(), bm);
        }
        Ok(())
    }
    #[track_caller]
    pub fn get_mut_buffer_modifier(&mut self, buffer: &api::Buffer) -> crate::Result<&mut BufferModifierGroupedUndo> {
        let Some(buffer_modifier) = self.buffer_modifiers.get_mut(buffer) else {
            return Err("BufferModifierGroupedUndo not created.".into());
        };
        Ok(buffer_modifier)
    }
    pub fn buffer_modifier_id_finished(&mut self, buffer: &api::Buffer, id: &usize) -> crate::Result<bool> {
        match self.buffer_modifiers.entry(buffer.clone()) {
            std::collections::hash_map::Entry::Occupied(mut bm_entry) => {
                if bm_entry.get_mut().id_finished(id) {
                    bm_entry.remove();
                    return Ok(true);
                }
            }
            _ => (),
        };
        Ok(false)
    }
}
