//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of `PID_ALLOCATOR` allocates pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.
mod context;
mod id;
mod manager;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE_BITS},
    loader::get_app_data_by_name,
    mm::{translated_byte_buffer, MapPermission},
    timer::get_time_us,
};
use alloc::sync::Arc;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager};
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use id::{kstack_alloc, pid_alloc, KernelStack, PidHandle};
pub use manager::add_task;
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
    Processor,
};
/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        panic!("All applications completed!");
    }

    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap()
    ));
}

///Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

/// Increase the syscall times of current `Running` task.
pub fn increase_current_syscall_times(syscall_id: usize) {
    let current = current_task().unwrap();
    let mut inner = current.inner_exclusive_access();
    inner.syscall_times[syscall_id] += 1;
}

/// TimeVal structure
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    /// seconds
    pub sec: usize,
    /// microseconds
    pub usec: usize,
}

/// Task information
#[allow(unused)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// Copy data from user to kernel
/// return the length of data copied
/// return -1 if failed
pub fn copy_to_user<T>(dst: *mut T, src: &T) -> isize {
    let token = current_user_token();

    let src_ptr = src as *const _ as *const u8;
    let dst_ptr = dst as *mut _ as *mut u8;

    let len = core::mem::size_of::<T>();
    let src_bytes: &[u8] = unsafe { core::slice::from_raw_parts(src_ptr, len) };

    let buffers = translated_byte_buffer(token, dst_ptr, len);

    let mut written_len = 0;
    for buffer in buffers {
        buffer.copy_from_slice(&src_bytes[written_len..written_len + buffer.len()]);
        written_len += buffer.len();
    }

    written_len as isize
}

/// Get the current task info
pub fn get_current_task_info(ti: *mut TaskInfo) {
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let status = inner.task_status;
    let syscall_times = inner.syscall_times;
    // calculate the total running time of task
    // as the difference between the current time and the start time
    // note that the time is in milliseconds
    let time = (get_time_us() - inner.start_time) / 1000;
    drop(inner);
    let task_info = TaskInfo {
        status,
        syscall_times,
        time,
    };

    copy_to_user(ti, &task_info);
}

/// Get the amount of available memory
pub fn memory_is_available(start: usize, len: usize) -> bool {
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    inner.memory_set.memory_is_avaiable(start, len)
}

/// Map a virtual address to a physical address
pub fn get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    copy_to_user(ts, &time_val);
    0
}

/// Map a virtual address to a physical address
pub fn mmap(start: usize, len: usize, port: usize) -> isize {
    if start & ((1 << PAGE_SIZE_BITS) - 1) != 0 {
        println!("start address is not page-aligned");
        return -1;
    }

    if port & !7 != 0 || port & 7 == 0 {
        println!("invalid port");
        return -1;
    }

    if !memory_is_available(start, len) {
        println!("memory is not available");
        return -1;
    }

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    let permission = MapPermission::U | MapPermission::from_bits((port as u8) << 1).unwrap();
    inner
        .memory_set
        .insert_framed_area(start.into(), (start + len).into(), permission);
    0
}

/// Unmap a virtual area
pub fn munmap(start: usize, len: usize) -> isize {
    if start & ((1 << PAGE_SIZE_BITS) - 1) != 0 {
        println!("start address is not page-aligned");
        return -1;
    }

    if memory_is_available(start, len) {
        println!("memory not mapped");
        return -1;
    }

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner
        .memory_set
        .remove_framed_area(start.into(), (start + len).into())
}
