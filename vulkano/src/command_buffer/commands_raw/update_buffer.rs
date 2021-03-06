// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::error;
use std::fmt;
use std::sync::Arc;

use buffer::Buffer;
use buffer::BufferAccess;
use buffer::TypedBuffer;
use buffer::TypedBufferAccess;
use buffer::BufferInner;
use command_buffer::CommandAddError;
use command_buffer::cb::AddCommand;
use command_buffer::cb::UnsafeCommandBufferBuilder;
use command_buffer::pool::CommandPool;
use device::Device;
use device::DeviceOwned;
use VulkanObject;
use VulkanPointers;
use vk;

/// Command that sets the content of a buffer to some data.
pub struct CmdUpdateBuffer<B, D> {
    // The buffer to update.
    buffer: B,
    // Raw buffer handle.
    buffer_handle: vk::Buffer,
    // Offset of the update.
    offset: vk::DeviceSize,
    // Size of the update.
    size: vk::DeviceSize,
    // The data to write to the buffer or a reference to it.
    data: D,
}

impl<B, D> CmdUpdateBuffer<B, D> {
    /// Builds a command that writes data to a buffer.
    ///
    /// If the size of the data and the size of the buffer mismatch, then only the intersection
    /// of both will be written.
    ///
    /// The size of the modification must not exceed 65536 bytes. The offset and size must be
    /// multiples of four.
    #[inline]
    pub fn new<P>(buffer: P, data: D) -> Result<CmdUpdateBuffer<B, D>, CmdUpdateBufferError>
        where P: Buffer<Access = B> + TypedBuffer<Content = D>,
              B: BufferAccess,
              D: 'static
    {
        unsafe {
            CmdUpdateBuffer::unchecked_type(buffer.access(), data)
        }
    }

    /// Same as `new`, except that the parameter is a `BufferAccess` instead of a `Buffer`.
    #[inline]
    pub fn from_access(buffer: B, data: D) -> Result<CmdUpdateBuffer<B, D>, CmdUpdateBufferError>
        where B: BufferAccess + TypedBufferAccess<Content = D>,
              D: 'static
    {
        unsafe {
            CmdUpdateBuffer::unchecked_type(buffer, data)
        }
    }

    /// Same as `from_access`, except that type safety is not enforced.
    pub unsafe fn unchecked_type(buffer: B, data: D)
                                 -> Result<CmdUpdateBuffer<B, D>, CmdUpdateBufferError>
        where B: BufferAccess
    {
        let size = buffer.size();

        let (buffer_handle, offset) = {
            let BufferInner { buffer: buffer_inner, offset } = buffer.inner();
            if !buffer_inner.usage_transfer_dest() {
                return Err(CmdUpdateBufferError::BufferMissingUsage);
            }
            if offset % 4 != 0 {
                return Err(CmdUpdateBufferError::WrongAlignment);
            }
            (buffer_inner.internal_object(), offset)
        };

        if size % 4 != 0 {
            return Err(CmdUpdateBufferError::WrongAlignment);
        }

        if size > 65536 {
            return Err(CmdUpdateBufferError::DataTooLarge);
        }

        Ok(CmdUpdateBuffer {
            buffer: buffer,
            buffer_handle: buffer_handle,
            offset: offset as vk::DeviceSize,
            size: size as vk::DeviceSize,
            data: data,
        })
    }

    /// Returns the buffer that is going to be written.
    #[inline]
    pub fn buffer(&self) -> &B {
        &self.buffer
    }
}

unsafe impl<B, D> DeviceOwned for CmdUpdateBuffer<B, D>
    where B: DeviceOwned
{
    #[inline]
    fn device(&self) -> &Arc<Device> {
        self.buffer.device()
    }
}

unsafe impl<'a, P, B, D> AddCommand<&'a CmdUpdateBuffer<B, D>> for UnsafeCommandBufferBuilder<P>
    where B: BufferAccess,
          P: CommandPool,
{
    type Out = UnsafeCommandBufferBuilder<P>;

    #[inline]
    fn add(self, command: &'a CmdUpdateBuffer<B, D>) -> Result<Self::Out, CommandAddError> {
        unsafe {
            let data = &command.data as *const D as *const _;
            let vk = self.device().pointers();
            let cmd = self.internal_object();
            vk.CmdUpdateBuffer(cmd, command.buffer_handle, command.offset, command.size, data);
        }

        Ok(self)
    }
}

/// Error that can happen when creating a `CmdUpdateBuffer`.
#[derive(Debug, Copy, Clone)]
pub enum CmdUpdateBufferError {
    /// The "transfer destination" usage must be enabled on the buffer.
    BufferMissingUsage,
    /// The data or size must be 4-bytes aligned.
    WrongAlignment,
    /// The data must not be larger than 64k bytes.
    DataTooLarge,
}

impl error::Error for CmdUpdateBufferError {
    #[inline]
    fn description(&self) -> &str {
        match *self {
            CmdUpdateBufferError::BufferMissingUsage => {
                "the transfer destination usage must be enabled on the buffer"
            },
            CmdUpdateBufferError::WrongAlignment => {
                "the offset or size are not aligned to 4 bytes"
            },
            CmdUpdateBufferError::DataTooLarge => "data is too large",
        }
    }
}

impl fmt::Display for CmdUpdateBufferError {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "{}", error::Error::description(self))
    }
}
