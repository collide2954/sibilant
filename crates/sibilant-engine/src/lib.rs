pub mod click;
pub mod engine;
pub mod timing;

#[cfg(test)]
#[global_allocator]
static ALLOCATOR: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
