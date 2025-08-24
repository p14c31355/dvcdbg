use heapless::Vec;
use embedded_hal::blocking::i2c::{Write, WriteRead};

pub struct Command<'a> {
    pub bytes: &'a [u8],
    pub depends_on: &'a [&'a [u8]],
}

pub struct Explorer<'a, I2C, const N: usize>
where
    I2C: Write,
{
    i2c: I2C,
    addr: u8,
    commands: &'a [Command<'a>],
    visited: Vec<&'a [u8], N>,
}

impl<'a, I2C, const N: usize> Explorer<'a, I2C, N>
where
    I2C: Write,
{
    pub fn new(i2c: I2C, addr: u8, commands: &'a [Command<'a>]) -> Self {
        Self {
            i2c,
            addr,
            commands,
            visited: Vec::new(),
        }
    }

    pub fn explore(&mut self) -> Result<(), I2C::Error> {
        for cmd in self.commands {
            self.try_send(cmd)?;
        }
        Ok(())
    }

    fn try_send(&mut self, cmd: &Command<'a>) -> Result<(), I2C::Error> {
        for dep in cmd.depends_on {
            if !self.visited.contains(dep) {
                let dep_cmd = self.commands.iter().find(|c| c.bytes == *dep).unwrap();
                self.try_send(dep_cmd)?;
            }
        }

        if self.visited.contains(&cmd.bytes) {
            return Ok(());
        }

        self.i2c.write(self.addr, cmd.bytes)?;

        self.visited.push(cmd.bytes).ok();
        Ok(())
    }
}
