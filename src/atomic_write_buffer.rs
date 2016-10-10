





#[derive(Debug)]
struct AtomicWriteBuffer<T: AsMut<[u8]>> {
    buffer: T,
    position: usize
}

impl<T::
