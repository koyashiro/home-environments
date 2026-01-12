#[cfg(feature = "btleplug")]
mod btleplug_ble_receiver;
#[cfg(all(feature = "mock", not(feature = "btleplug")))]
mod mock_ble_receiver;

#[cfg(feature = "btleplug")]
pub use btleplug_ble_receiver::ble_receiver;
#[cfg(all(feature = "mock", not(feature = "btleplug")))]
pub use mock_ble_receiver::ble_receiver;
