use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering}, Arc, Condvar, Mutex
    }, thread, time::Duration,
};

///Manage a collection of threads
///
/// A new thread is created every time all the existing threads are full.
/// Any idle thread will automatically die after a few threads
pub struct TaskPool {
    sharing: Arc<Sharing>
}

struct Sharing {
    //list of the tasks to be done by workers threads
    todo: Mutex<VecDeque<Box<dyn FnMut() + Send>>>,

    // condvar that will be notified whenever a task is added to `todo`
    condvar: Condvar,

    //number of total worker threads running
    active_workers: AtomicUsize,

    //number of idle worker threads
    idle_workers: AtomicUsize,
}

///Minimum number of active threads
static MIN_THREADS: usize = 4;

struct Registration<'a> {
    nb: &'a AtomicUsize,
}

impl<'a> Registration<'a> {
    fn new(nb: &'a AtomicUsize) -> Registration<'a> {
        nb.fetch_add(1, Ordering::Release);
        Registration { nb }
    }
}

impl<'a> Drop for Registration<'a> {
    fn drop(&mut self) {
        self.nb.fetch_sub(1, Ordering::Release);
    }
}

impl TaskPool {
    
    pub fn new() -> TaskPool {
        let pool = TaskPool {
            sharing: Arc::new(
            Sharing {
                todo: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                active_workers: AtomicUsize::new(0),
                idle_workers: AtomicUsize::new(0)
            }
        )};

        for _ in 0..MIN_THREADS {
            pool.add_thread(None);
        }

        pool
    }

    ///Executes a function in a thread
    /// If no thread is available, spawns new one
    pub fn spawn(&self, code: Box<dyn FnMut() + Send>) {
        let mut queue = self.sharing.todo.lock().unwrap();

        if self.sharing.idle_workers.load(Ordering::Acquire) == 0 {
            self.add_thread(Some(code));
        } else {
            queue.push_back(code);
            self.sharing.condvar.notify_one();
        }
    }

    fn add_thread(&self, intial_fn: Option<Box<dyn FnMut() + Send>>) {

        let sharing = self.sharing.clone();

        thread::spawn(move || {
            //存活线程加1
            let _active_guard = Registration::new(&sharing.active_workers);

            //执行线程初始函数
            if let Some(mut f) = intial_fn {
                f();
            }

            loop {
                //循环，添加的线程执行完初始函数，从任务对列里取任务执行
                let mut task: Box<dyn FnMut() + Send> = {
                    let mut todo = sharing.todo.lock().unwrap();
                    let task;
                    loop {
                        //取任务队列的任务
                        if let Some(pop_task) = todo.pop_front() {
                            task = pop_task;
                            break;
                        }
                        //任务队列为空
                        let _idle_guard = Registration::new(&sharing.idle_workers);
                        let received = if sharing.active_workers.load(Ordering::Relaxed) <= MIN_THREADS {
                            todo = sharing
                                .condvar
                                .wait(todo)
                                .unwrap();
                            true
                        } else {
                            let (new_todo, waitres) = sharing
                                .condvar
                                .wait_timeout(todo, Duration::from_millis(5000))
                                .unwrap();
                            todo = new_todo;
                            !waitres.timed_out()
                        };
                        if !received && todo.is_empty() {
                            //当不是要一直存在的线程，在5000ms内获取不到任务，结束线程
                            return;
                        }
                    }
                    task
                };
                //执行任务
                task();
            }
        });
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.sharing
            .active_workers
            .store(999_999_999, Ordering::Release);
        self.sharing.condvar.notify_all();
    }
}

