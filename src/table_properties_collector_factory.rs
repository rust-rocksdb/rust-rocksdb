use libc::{c_void, c_char, uint32_t};
use table_properties_collector::{TablePropertiesCollector, new_table_properties_collector_context};

pub trait TablePropertiesCollectorFactory {
    fn name(&self) -> &str;
    fn create_table_properties_collector(&mut self, cf: u32) -> Box<TablePropertiesCollector>;
}

#[repr(C)]
pub struct TablePropertiesCollectorFactoryContext {
    factory: *mut c_void,
    name: extern "C" fn(*mut c_void) -> *const c_char,
    destructor: extern "C" fn(*mut c_void),
    create_table_properties_collector: extern "C" fn(*mut c_void, uint32_t) -> *mut c_void,
}

extern "C" fn name(context: *mut c_void) -> *const c_char {
    unsafe {
        let context = &mut *(context as *mut TablePropertiesCollectorFactoryContext);
        let factory = &mut *(context.factory as *mut Box<TablePropertiesCollectorFactory>);
        factory.name().as_ptr() as *const c_char
    }
}

extern "C" fn destructor(context: *mut c_void) {
    unsafe {
        let context = Box::from_raw(context as *mut TablePropertiesCollectorFactoryContext);
        Box::from_raw(context.factory as *mut Box<TablePropertiesCollectorFactory>);
    }
}

extern "C" fn create_table_properties_collector(context: *mut c_void, cf: uint32_t) -> *mut c_void {
    unsafe {
        let context = &mut *(context as *mut TablePropertiesCollectorFactoryContext);
        let factory = &mut *(context.factory as *mut Box<TablePropertiesCollectorFactory>);
        let collector = factory.create_table_properties_collector(cf);
        Box::into_raw(new_table_properties_collector_context(collector)) as *mut c_void
    }
}

pub unsafe fn new_table_properties_collector_factory_context
    (factory: Box<TablePropertiesCollectorFactory>)
     -> Box<TablePropertiesCollectorFactoryContext> {
    Box::new(TablePropertiesCollectorFactoryContext {
        factory: Box::into_raw(Box::new(factory)) as *mut c_void,
        name: name,
        destructor: destructor,
        create_table_properties_collector: create_table_properties_collector,
    })
}
