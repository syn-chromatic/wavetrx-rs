use std::error;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use cpal::traits::DeviceTrait;
use cpal::traits::StreamTrait;
use cpal::BuildStreamError;
use cpal::Device;
use cpal::InputCallbackInfo;
use cpal::Stream;
use cpal::StreamConfig;
use cpal::StreamError;