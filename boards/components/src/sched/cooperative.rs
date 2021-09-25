//! Component for a cooperative scheduler.
//!
//! This provides one Component, CooperativeComponent.
//!
//! Usage
//! -----
//! ```rust
//! let scheduler = components::cooperative::CooperativeComponent::new(&PROCESSES)
//!     .finalize(components::coop_component_helper!(NUM_PROCS));
//! ```

// Author: Hudson Ayers <hayers@stanford.edu>

use core::mem::MaybeUninit;
use kernel::collections::list::simple_linked_list::{SimpleLinkedList, SimpleLinkedListNode};
use kernel::collections::list::SinglyLinkedList;
use kernel::component::Component;
use kernel::process::Process;
use kernel::scheduler::cooperative::CooperativeSched;
use kernel::{static_init, static_init_half};

#[macro_export]
macro_rules! coop_component_helper {
    ($N:expr $(,)?) => {{
        use core::mem::MaybeUninit;
        use kernel::collections::list::simple_linked_list::SimpleLinkedListNode;
        use kernel::process::Process;
        use kernel::static_buf;
        const UNINIT: MaybeUninit<SimpleLinkedListNode<'static, Option<&'static dyn Process>>> =
            MaybeUninit::uninit();
        static mut BUF: [MaybeUninit<SimpleLinkedListNode<'static, Option<&'static dyn Process>>>;
            $N] = [UNINIT; $N];
        &mut BUF
    };};
}

pub type SchedulerType = CooperativeSched<
        'static,
        SimpleLinkedListNode<'static, Option<&'static dyn Process>>,
        SimpleLinkedList<'static, Option<&'static dyn Process>>,
    >;

pub struct CooperativeComponent {
    processes: &'static [Option<&'static dyn Process>],
}

impl CooperativeComponent {
    pub fn new(processes: &'static [Option<&'static dyn Process>]) -> CooperativeComponent {
        CooperativeComponent { processes }
    }
}

impl Component for CooperativeComponent {
    type StaticInput =
        &'static mut [MaybeUninit<SimpleLinkedListNode<'static, Option<&'static dyn Process>>>];
    type Output = &'static mut SchedulerType;

    unsafe fn finalize(self, proc_nodes: Self::StaticInput) -> Self::Output {
        let scheduler = static_init!(
            SchedulerType,
            CooperativeSched::new(SimpleLinkedList::new())
        );

        for (i, node) in proc_nodes.iter_mut().enumerate() {
            let init_node = static_init_half!(
                node,
                SimpleLinkedListNode<'static, Option<&'static dyn Process>>,
                SimpleLinkedListNode::new(self.processes[i])
            );
            scheduler.processes.push_head(init_node);
        }
        scheduler
    }
}
