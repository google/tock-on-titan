
#[macro_export]
macro_rules! interrupt_handler {
    ($name: ident, $nvic: expr $(, $body: expr)*) => {
        #[no_mangle]
        #[allow(non_snake_case)]
        #[allow(unused_imports)]
        pub unsafe extern fn $name() {
            $({
                $body
            })*

            ::cortexm3::nvic::Nvic::new($nvic).disable();
        }
    }
}

