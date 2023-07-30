use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(FromPrimitive)]
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum PacketType 
{
    IDENTITY,
    SERVER_IDENTITY_RESPONSE,

    SERVER_PLAYER_JOINED,
    SERVER_PLAYER_LEFT,
    SERVER_PLAYER_INFO,
    SERVER_PLAYER_FORCE_DISCONNECT,

    SERVER_LOBBY_READY_STATE,
    SERVER_LOBBY_EXE,
    SERVER_LOBBY_COUNTDOWN,
    SERVER_LOBBY_EXE_CHANGE,
    SERVER_LOBBY_CHARACTER_CHANGE,
    SERVER_LOBBY_CHARACTER_RESPONSE,
    SERVER_LOBBY_EXECHARACTER_RESPONSE,
    SERVER_LOBBY_GAME_START,
    SERVER_LOBBY_PLAYER,
    SERVER_LOBBY_EXE_CHANCE,
    SERVER_LOBBY_CORRECT,
    SERVER_LOBBY_VOTEKICK,
    SERVER_CHAR_TIME_SYNC,

    SERVER_VOTE_MAPS,
    SERVER_VOTE_SET,
    SERVER_VOTE_TIME_SYNC,

    SERVER_GAME_PLAYERS_READY,
    SERVER_GAME_EXE_WINS,
    SERVER_GAME_SURVIVOR_WIN,
    SERVER_GAME_SPAWN_RING,
    SERVER_GAME_PLAYER_ESCAPED,
    SERVER_GAME_BACK_TO_LOBBY,
    SERVER_GAME_TIME_SYNC,
    SERVER_GAME_TIME_OVER,
    SERVER_GAME_PING,
    SERVER_PLAYER_DEATH_STATE,
    SERVER_GAME_DEATHTIMER_TICK,
    SERVER_GAME_DEATHTIMER_END,

    SERVER_REQUEST_INFO,
    SERVER_HEARTBEAT,
    SERVER_PONG,

    SERVER_FORCE_DAMAGE,
    SERVER_GAME_RING_READY,
    SERVER_PLAYER_BACKTRACK,

    // Entities
    SERVER_TPROJECTILE_STATE,
    SERVER_ETRACKER_STATE,
    SERVER_ERECTOR_BRING_SPAWN,
    SERVER_RMZSLIME_STATE,
    SERVER_RMZSLIME_RINGBONUS,
    SERVER_RMZSHARD_STATE,
    SERVER_LCEYE_STATE,
    SERVER_LCCHAIN_STATE,
    SERVER_NPCONTROLLER_STATE,
    SERVER_KAFMONITOR_STATE,
    SERVER_YCRSMOKE_STATE,
    SERVER_YCRSMOKE_READY,
    SERVER_MOVINGSPIKE_STATE,
    SERVER_RING_STATE,
    SERVER_RING_COLLECTED,
    SERVER_ACT9WALL_STATE,
    SERVER_NAPBALL_STATE,
    SERVER_NAPICE_STATE,
    SERVER_PFLIFT_STATE,
    SERVER_BRING_STATE,
    SERVER_BRING_COLLECTED,
    SERVER_VVLCOLUMN_STATE,
    SERVER_VVVASE_STATE,
    SERVER_GHZTHUNDER_STATE,
    SERVER_TCGOM_STATE,
    SERVER_EXELLERCLONE_STATE,
    SERVER_DTTAILSDOLL_STATE,
    SERVER_DTBALL_STATE,
    SERVER_DTASS_STATE,
    SERVER_HDDOOR_STATE,
    SERVER_FART_STATE,

    // Entity actions
    CLIENT_ETRACKER,
    CLIENT_ETRACKER_ACTIVATED,
    CLIENT_TPROJECTILE,
    CLIENT_TPROJECTILE_HIT,
    CLIENT_ERECTOR_BALLS,
    CLIENT_ERECTOR_BRING_SPAWN,
    CLIENT_EXELLER_SPAWN_CLONE,
    CLIENT_EXELLER_TELEPORT_CLONE,
    CLIENT_MERCOIN_BONUS,

    CLIENT_RMZSLIME_HIT,
    CLIENT_LCEYE_REQUEST_ACTIVATE,
    CLIENT_KAFMONITOR_ACTIVATE,
    CLIENT_RING_COLLECTED,
    CLIENT_BRING_COLLECTED,
    CLIENT_NAPICE_ACTIVATE,
    CLIENT_SPRING_USE,
    CLIENT_PFLIT_ACTIVATE,
    CLIENT_VVVASE_BREAK,
    CLIENT_RMZSHARD_COLLECT,
    CLIENT_RMZSHARD_LAND,
    CLIENT_DTASS_ACTIVATE,
    CLIENT_HDDOOR_TOGGLE,
    CLIENT_FART_PUSH,

    CLIENT_LOBBY_READY_STATE,
    CLIENT_REQUESTED_INFO,
    CLIENT_PLAYER_DATA,
    CLIENT_PLAYER_HURT,
    CLIENT_SOUND_EMIT,
    CLIENT_PING,

    CLIENT_REVIVAL_PROGRESS,
    CLIENT_PLAYER_HEAL,
    CLIENT_PLAYER_HEAL_PART,
    SERVER_REVIVAL_PROGRESS,
    SERVER_REVIVAL_STATUS,
    SERVER_REVIVAL_RINGSUB,
    SERVER_REVIVAL_REVIVED,

    CLIENT_REQUEST_CHARACTER,
    CLIENT_REQUEST_EXECHARACTER,
    CLIENT_VOTE_REQUEST,

    CLIENT_PLAYER_DEATH_STATE,
    CLIENT_PLAYER_ESCAPED,
    CLIENT_LOBBY_PLAYERS_REQUEST,
    CLIENT_CREAM_SPAWN_RINGS,
    CLIENT_SPAWN_EFFECT,
    CLIENT_CHAT_MESSAGE,
    CLIENT_LOBBY_VOTEKICK,
    CLIENT_PLAYER_PALLETE,
    CLIENT_PET_PALLETE,

    CLIENT_PLAYER_POTATER,
}


pub(crate) struct Packet
{
    buffer: Vec<u8>,
    position: usize
}

impl Packet
{
    pub fn new(t: PacketType) -> Packet
    {
        let mut pack = Packet { buffer: Vec::new(), position: 3 };
        pack.wu8(0); // Passtrough
        pack.wpk(t); // Type

        pack
    }

    pub fn from(arr: &[u8], size: usize) -> Packet
    {
        let mut pack = Packet { buffer: vec![0; size], position: 0 };
        pack.buffer.copy_from_slice(&arr[..size]);

        pack
    }


    pub fn wpk(&mut self, val: PacketType)
    {
        self.buffer.push(val as u8);
    }

    pub fn wu8(&mut self, val: u8)
    {
        self.buffer.push(val);
    }

    pub fn wi8(&mut self, val: i8)
    {
        self.buffer.push(val as u8);
    }

    pub fn wu16(&mut self, val: u16)
    {
        let bytes = val.to_le_bytes();

        for val in bytes
        {
            self.buffer.push(val);
        }
    }

    pub fn wi16(&mut self, val: i16)
    {
        let bytes = val.to_le_bytes();

        for val in bytes
        {
            self.buffer.push(val);
        }
    }

    pub fn wu32(&mut self, val: u32)
    {
        let bytes = val.to_le_bytes();

        for val in bytes
        {
            self.buffer.push(val);
        }
    }

    pub fn wi32(&mut self, val: i32)
    {
        let bytes = val.to_le_bytes();

        for val in bytes
        {
            self.buffer.push(val);
        }
    }

    pub fn wu64(&mut self, val: u64)
    {
        let bytes = val.to_le_bytes();

        for val in bytes
        {
            self.buffer.push(val);
        }
    }

    pub fn wi64(&mut self, val: i64)
    {
        let bytes = val.to_le_bytes();

        for val in bytes
        {
            self.buffer.push(val);
        }
    }


    pub fn wf32(&mut self, val: f32)
    {
        let bytes = val.to_le_bytes();

        for val in bytes
        {
            self.buffer.push(val);
        }
    }

    pub fn wf64(&mut self, val: f64)
    {
        let bytes = val.to_le_bytes();

        for val in bytes
        {
            self.buffer.push(val);
        }
    }

    pub fn wstr(&mut self, val: &str)
    {
        let bytes = val.bytes();

        for val in bytes
        {
            self.buffer.push(val);
        }

        self.buffer.push('\0' as u8);

    }

    pub fn rpk(&mut self) -> PacketType
    {
        let val = self.buffer[self.position];
        self.position += 1;

        FromPrimitive::from_u8(val).expect("Failed to convert to PacketType")
    }

    pub fn ru8(&mut self) -> u8
    {
        let val = self.buffer[self.position];
        self.position += 1;
        val
    }

    pub fn ri8(&mut self) -> i8
    {
        let val = self.buffer[self.position] as i8;
        self.position += 1;
        val
    }

    pub fn ru16(&mut self) -> u16
    {
        let input: [u8; 2] = match self.buffer[self.position..self.position+2].try_into()
        {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to convert: {}", err);
            }
        };

        self.position += 2;
        u16::from_le_bytes(input)
    }

    pub fn ri16(&mut self) -> i16
    {
        let input: [u8; 2] = match self.buffer[self.position..self.position+2].try_into()
        {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to convert: {}", err);
            }
        };

        self.position += 2;
        i16::from_le_bytes(input)
    }

    pub fn ru32(&mut self) -> u32
    {
        let input: [u8; 4] = match self.buffer[self.position..self.position+4].try_into()
        {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to convert: {}", err);
            }
        };

        self.position += 4;
        u32::from_le_bytes(input)
    }

    pub fn ri32(&mut self) -> i32
    {
        let input: [u8; 4] = match self.buffer[self.position..self.position+4].try_into()
        {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to convert: {}", err);
            }
        };

        self.position += 4;
        i32::from_le_bytes(input)
    }

    pub fn ru64(&mut self) -> u64
    {
        let input: [u8; 8] = match self.buffer[self.position..self.position+8].try_into()
        {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to convert: {}", err);
            }
        };

        self.position += 8;
        u64::from_le_bytes(input)
    }

    pub fn ri64(&mut self) -> i64
    {
        let input: [u8; 8] = match self.buffer[self.position..self.position+8].try_into()
        {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to convert: {}", err);
            }
        };

        self.position += 8;
        i64::from_le_bytes(input)
    }

    pub fn rf32(&mut self) -> f32
    {
        let input: [u8; 4] = match self.buffer[self.position..self.position+4].try_into()
        {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to convert: {}", err);
            }
        };

        self.position += 4;
        f32::from_le_bytes(input)
    }

    pub fn rf64(&mut self) -> f64
    {
        let input: [u8; 8] = match self.buffer[self.position..self.position+8].try_into()
        {
            Ok(res) => res,
            Err(err) => {
                panic!("Failed to convert: {}", err);
            }
        };

        self.position += 8;
        f64::from_le_bytes(input)
    }

    pub fn rstr(&mut self) -> String
    {
        let mut str = String::new();
        let mut ch = self.buffer[self.position] as char;
        self.position += 1;

        while ch != '\0'
        {
            str.push(ch);

            ch = self.buffer[self.position] as char;
            self.position += 1;
        }

        str
    }

    pub fn buf(&mut self) -> &[u8]
    {
        self.buffer.insert(0, 0);
        self.position += 1; // move by one

        self.buffer[0] = (self.buffer.len() - 1) as u8; // Set final length
        &self.buffer
    }

}