// This is free and unencumbered software released into the public domain.

use crate::{
    prelude::{
        Arc, AtomicBool, AtomicUsize, Box, Duration, Instant, Ordering, Range, Rc, RefCell,
        ToString, Vec,
    },
    transport::Transport,
    Block, BlockError, BlockResult, BlockRuntime, Port, Process, ProcessID, Runtime, System,
};

#[cfg(feature = "std")]
extern crate std;

#[allow(unused)]
pub struct StdRuntime {
    transport: Box<dyn Transport>,
    is_alive: AtomicBool,
    process_id: AtomicUsize,
}

#[allow(unused)]
impl StdRuntime {
    pub fn new(transport: Box<dyn Transport>) -> Result<Arc<Self>, BlockError> {
        Ok(Arc::new(Self {
            transport,
            is_alive: AtomicBool::new(true),
            process_id: AtomicUsize::new(1),
        }))
    }
}

impl Runtime for Arc<StdRuntime> {
    fn execute_block(&mut self, block: Box<dyn Block>) -> BlockResult<Rc<dyn Process>> {
        let block_runtime = Arc::new((*self).clone()) as Arc<dyn BlockRuntime>;
        let block_process = Rc::new(RunningBlock {
            id: self.process_id.fetch_add(1, Ordering::SeqCst),
            runtime: self.clone(),
            handle: RefCell::new(Some(
                std::thread::Builder::new()
                    .name(block.name().unwrap_or_else(|| "<unnamed>".to_string()))
                    .spawn(move || {
                        let mut block = block;
                        std::thread::park();
                        Block::prepare(block.as_mut(), block_runtime.as_ref()).unwrap();
                        Block::execute(block.as_mut(), block_runtime.as_ref()).unwrap();
                        ()
                    })
                    .unwrap(),
            )),
        });
        block_process
            .handle
            .borrow()
            .as_ref()
            .unwrap()
            .thread()
            .unpark();
        Ok(block_process)
    }

    fn execute_system(&mut self, system: System) -> BlockResult<Rc<dyn Process>> {
        let mut system_process = RunningSystem {
            id: self.process_id.fetch_add(1, Ordering::SeqCst),
            runtime: self.clone(),
            blocks: Vec::new(),
        };

        for (source_id, target_id) in system.connections.borrow().iter() {
            self.transport.connect(*source_id, *target_id).unwrap();
        }

        while let Some(block) = system.blocks.borrow_mut().pop_front() {
            system_process.blocks.push(self.execute_block(block)?);
        }

        Ok(Rc::new(system_process))
    }
}

impl BlockRuntime for Arc<StdRuntime> {
    fn is_alive(&self) -> bool {
        self.is_alive.load(Ordering::SeqCst)
    }

    fn sleep_for(&self, duration: Duration) -> Result<(), BlockError> {
        #[cfg(feature = "std")]
        std::thread::sleep(duration);
        #[cfg(not(feature = "std"))]
        unimplemented!("std::thread::sleep requires the 'std' feature");
        Ok(())
    }

    fn sleep_until(&self, _instant: Instant) -> Result<(), BlockError> {
        todo!() // TODO
    }

    fn wait_for(&self, _port: &dyn Port) -> Result<(), BlockError> {
        // while self.is_alive() && !port.is_connected() {
        //     self.yield_now()?;
        // }
        // if self.is_alive() {
        //     Ok(())
        // } else {
        //     Err(BlockError::Terminated)
        // }
        Ok(()) // TODO
    }

    fn yield_now(&self) -> Result<(), BlockError> {
        #[cfg(feature = "std")]
        std::thread::yield_now();
        #[cfg(not(feature = "std"))]
        unimplemented!("std::thread::yield_now requires the 'std' feature");
        Ok(())
    }

    fn random_duration(&self, range: Range<Duration>) -> Duration {
        #[cfg(all(feature = "std", feature = "rand"))]
        {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let low = range.start.as_nanos() as u64;
            let high = range.end.as_nanos() as u64;
            Duration::from_nanos(rng.gen_range(low..high))
        }
        #[cfg(not(all(feature = "std", feature = "rand")))]
        let mut _rng = todo!();
    }
}

#[allow(unused)]
struct RunningBlock {
    id: ProcessID,
    runtime: Arc<StdRuntime>,
    handle: RefCell<Option<std::thread::JoinHandle<()>>>,
}

#[allow(unused)]
impl RunningBlock {
    //fn thread(&self) -> Option<&std::thread::Thread> {
    //    self.handle.borrow().as_ref().map(|handle| handle.thread())
    //}
}

impl Process for RunningBlock {
    fn id(&self) -> ProcessID {
        self.id
    }

    fn is_alive(&self) -> bool {
        self.handle
            .borrow()
            .as_ref()
            .map(|handle| !handle.is_finished())
            .unwrap_or(false)
    }

    fn join(&self) -> Result<(), ()> {
        self.handle.take().unwrap().join().map_err(|_| ())
    }
}

#[allow(unused)]
struct RunningSystem {
    id: ProcessID,
    runtime: Arc<StdRuntime>,
    blocks: Vec<Rc<dyn Process>>,
}

impl Process for RunningSystem {
    fn id(&self) -> ProcessID {
        self.id
    }

    fn is_alive(&self) -> bool {
        self.blocks.iter().any(|block| block.is_alive())
    }

    fn join(&self) -> Result<(), ()> {
        for block in self.blocks.iter() {
            block.join()?;
        }
        Ok(())
    }
}