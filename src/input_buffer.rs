#[derive(Debug)]
pub enum AddError
{
    Overflow
}

/// InputBuffer is a byte buffer with a fixed capacity
/// but dynamic lenth.
/// Adding data will always grow `len()`
/// towards capacity, while consuming data will always
/// take data from start up to the number of bytes
/// consumed, moving all data left to start.
///
/// For instance if the content of the buffer is `[0, 1, 2, 3, 4]`
/// and you consume 2 bytes of data, you will consume `[0, 1]`
/// and the content of the buffer will be `[2, 3, 4]`.
pub struct InputBuffer<'a> {
    buffer: &'a mut [u8],
    next_input_pos: usize,
    overflow: bool
}

impl<'a> InputBuffer<'a> {
    fn write_area(&mut self) -> &mut [u8] {
        return self.buffer.split_at_mut(self.next_input_pos).1;
    }
    /// Returns an InputBuffer using the designated backing buffer
    ///
    /// # Arguments
    ///
    /// * `buffer` the backing buffer used to provide storage for the InputBuffer.
    pub fn new(buffer: &'a mut [u8]) -> InputBuffer {
        InputBuffer {
            buffer: buffer,
            next_input_pos: 0,
            overflow: false,
        }
    }

    /// Push data to the back of the buffer.
    ///
    /// # Arguments
    ///
    /// * `value` the data to push
    ///
    /// # Overflow behaviour
    ///
    /// On overflow an error is returned, and `x.overflown()` will return true
    pub fn push(&mut self, value: u8) -> Result<(), AddError> {
        if self.next_input_pos < self.capacity() {
            self.buffer[self.next_input_pos as usize] = value;
            self.next_input_pos += 1;
            Ok(())
        }
        else {
            self.overflow = true;
            Err(AddError::Overflow)
        }
    }

    pub fn push_multiple(&mut self, values: &[u8]) -> usize {
        let available_space = self.capacity() - self.len();
        if values.len() <= available_space {
            self.write_area().split_at_mut(values.len()).0.copy_from_slice(values);
            self.next_input_pos += values.len();
            return values.len();
        }
        else {
            self.write_area().copy_from_slice(values.split_at(available_space).0);
            self.overflow = true;
            return available_space;
        }
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    pub fn len(&self) -> usize {
        self.next_input_pos
    }

    pub fn is_empty(&self) -> bool {
        self.next_input_pos == 0
    }

    /// Returns true if an overflow has been detected
    ///
    /// Use `ib.clear()` to clear the overflow flag.
    pub fn overflown(&self) -> bool {
        self.overflow
    }

    /// Resizes the buffer.
    ///
    /// This does *not* clear the overflow flag,
    /// use `ib.clear()` for that.
    ///
    /// If `new_size` is greater than capacity then
    /// the value of capacity is used instead.
    pub fn resize(&mut self, new_size: usize) {
        self.next_input_pos = new_size.min(self.capacity());
    }

    /// Clears the buffer.
    ///
    /// This clears the overflow flag and
    /// sets len to 0.
    pub fn clear(&mut self) {
        self.next_input_pos = 0;
        self.overflow = false;
    }

    /// Takes data from the start of the buffer, moving any
    /// remaining data to the start of the buffer and decreases
    /// len.
    ///
    /// This is the only way to retrieve data from the buffer.
    ///
    /// # Arguments
    ///
    /// * `output` The output buffer for data.
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate uio_buffer;
    /// use uio_buffer::input_buffer;
    /// let mut backing_buffer = [0u8; 10];
    /// let mut buffer = input_buffer::InputBuffer::new(&mut backing_buffer);
    /// buffer.push_multiple(&[1,2,3,4,5]);
    /// assert_eq!(buffer.len(), 5);
    /// let mut read_buffer = [0u8; 3];
    /// let consumed = buffer.consume(&mut read_buffer);
    /// assert_eq!(consumed, 3);
    /// assert_eq!(read_buffer, [1,2,3]);
    /// assert_eq!(buffer.len(), 2);
    /// // buffer contains [4,5]
    /// ```
    pub fn consume(&mut self, output: &mut [u8]) -> usize {
        let bytes_to_consume = self.len().min(output.len());
        if bytes_to_consume == 0 {
            return 0;
        }

        let split_input = self.buffer.split_at(bytes_to_consume);
        let output_area = output.split_at_mut(bytes_to_consume).0;
        let input_area = split_input.0;
        output_area.copy_from_slice(input_area);

        let new_len = self.len()-bytes_to_consume;
        if new_len == 0 {
            self.next_input_pos = 0;
            return bytes_to_consume;
        }
        self.buffer.copy_within(core::ops::Range{start: bytes_to_consume, end: self.next_input_pos}, 0);
        self.next_input_pos = new_len;
        return bytes_to_consume;
    }
}

mod tests {
    use super::InputBuffer;

    #[test]
    fn capacity_len_on_new() {
        let mut buffer = [0u8; 10];
        let target_len = buffer.len();

        let input_buffer = InputBuffer::new(&mut buffer);
        assert_eq!(input_buffer.capacity(), target_len);
        assert_eq!(input_buffer.len(), 0);
    }

    #[test]
    fn len_increases_on_add() {
        let mut buffer = [0u8; 10];

        {
            let mut input_buffer = InputBuffer::new(&mut buffer);
            input_buffer.push(10).unwrap();
            assert_eq!(input_buffer.len(), 1);

            input_buffer.push(20).unwrap();
            assert_eq!(input_buffer.len(), 2);
        }

        assert_eq!(buffer[0], 10);
        assert_eq!(buffer[1], 20);
    }

    #[test]
    fn detect_overflow()
    {
        let mut buffer = [0u8; 2];

        {
            let mut input_buffer = InputBuffer::new(&mut buffer);
            input_buffer.push(10).unwrap();
            input_buffer.push(20).unwrap();

            assert!(!input_buffer.overflown());
            assert!(input_buffer.push(30).is_err());
            assert!(input_buffer.overflown());

            input_buffer.clear();
            assert!(!input_buffer.overflown());
            assert_eq!(input_buffer.len(), 0);
        }

        assert_eq!(buffer[0], 10);
        assert_eq!(buffer[1], 20);
    }

    #[test]
    fn consume() {
        let mut buffer = [0u8; 10];
        let mut input_buffer = InputBuffer::new(&mut buffer);

        let mut consume_buffer = [0u8; 2];

        input_buffer.push(1).unwrap();
        assert_eq!(input_buffer.len(), 1);
        let consumed = input_buffer.consume(&mut consume_buffer);
        assert_eq!(consumed, 1);
        assert_eq!(consume_buffer[0], 1);
        assert_eq!(input_buffer.len(), 0);

        input_buffer.push(2).unwrap();
        input_buffer.push(3).unwrap();
        let consumed = input_buffer.consume(&mut consume_buffer);
        assert_eq!(consumed, 2);
        assert_eq!(consume_buffer[0], 2);
        assert_eq!(consume_buffer[1], 3);
        assert_eq!(input_buffer.len(), 0);
        
        input_buffer.push(4).unwrap();
        input_buffer.push(5).unwrap();
        input_buffer.push(6).unwrap();
        let consumed = input_buffer.consume(&mut consume_buffer);
        assert_eq!(consumed, 2);
        assert_eq!(consume_buffer[0], 4);
        assert_eq!(consume_buffer[1], 5);
        assert_eq!(input_buffer.len(), 1);

        let consumed = input_buffer.consume(&mut consume_buffer);
        assert_eq!(consumed, 1);
        assert_eq!(consume_buffer[0], 6);
        assert_eq!(input_buffer.len(), 0);

        input_buffer.push(4).unwrap();
        input_buffer.push(5).unwrap();
        input_buffer.push(6).unwrap();
        let consumed = input_buffer.consume(&mut consume_buffer);
        assert_eq!(consumed, 2);
        assert_eq!(consume_buffer[0], 4);
        assert_eq!(consume_buffer[1], 5);
        assert_eq!(input_buffer.len(), 1);

        input_buffer.push(7).unwrap();
        let consumed = input_buffer.consume(&mut consume_buffer);
        assert_eq!(consumed, 2);
        assert_eq!(consume_buffer[0], 6);
        assert_eq!(consume_buffer[1], 7);
        assert_eq!(input_buffer.len(), 0);
    }
}