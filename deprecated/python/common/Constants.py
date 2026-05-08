import Atrea.enums

# Skin tints used for character creation
SKIN_TINTS = [
	0x2F1308FF, 0x180A08FF, 0x15100DFF, 0x9C4F22FF,
	0x370405FF, 0x2F1219FF, 0x6C1F0DFF, 0x4F1A09FF,
	0xB45B32FF, 0x632319FF, 0x3A2417FF, 0xF8B487FF,
	0xD57D51FF, 0xC36141FF, 0xDF8250FF, 0x8D3F24FF
]

# Result codes passed to the callback registered by registerMinigameSession()
MINIGAME_RESULT_Canceled = 0
MINIGAME_RESULT_Victory = 1
MINIGAME_RESULT_Defeat = 2
MINIGAME_RESULT_NotStarted = 3

# Mission status codes
MISSION_Not_Active = 0
MISSION_Active = 1
MISSION_Completed = 2
MISSION_Failed = 3

# First channel ID that is dynamically allocated
MIN_USER_CHANNEL = Atrea.enums.CHAN_chat

# Messages to this channel are handled on the cell
CHANNEL_FLAG_OnCell = 1
# Players can't send messages on this channel
CHANNEL_FLAG_DisallowPlayerMessages = 2
# Don't delete channel when its empty
CHANNEL_FLAG_KeepIfEmpty = 4

# Health of entity cannot be damaged
CHEAT_Invulnerable = 1

# Safety valve in case player isn't turned visible (works around an SGW bug)
SEQUENCE_VisibilitySafety = 512

# Ability ID of reload ability
ABILITY_ReloadWeapon = 596

# Interaction set maps for each static interaction action
INTERACTION_OpenVault = 7514
INTERACTION_RegisterPvP = 7449
INTERACTION_DHD = 6891
INTERACTION_RingTransport = 7457
INTERACTION_OrganizationRegisterTeam = 7447
INTERACTION_OrganizationRegisterCommand = 7448
INTERACTION_VendorArmor = 7012
INTERACTION_VendorWeapons = 7006
INTERACTION_VendorConsumables = 7005
INTERACTION_VendorGeneral = 7505
INTERACTION_Loot = 20

INTERACTION_Trainer = {
	Atrea.enums.ARCHETYPE_Soldier: 3851,
	Atrea.enums.ARCHETYPE_Commando: 3887,
	Atrea.enums.ARCHETYPE_Scientist: 3888,
	Atrea.enums.ARCHETYPE_Archeologist: 3889,
	Atrea.enums.ARCHETYPE_Asgard: 6976,
	Atrea.enums.ARCHETYPE_Goauld: 0,
	Atrea.enums.ARCHETYPE_Sholva: 6975,
	Atrea.enums.ARCHETYPE_Jaffa: 6975
}

# InteractionSetMapId -> Class mappings for static interaction sets
STATIC_INTERACTIONS = {
	6891: 'DHD',
	7012: 'Vendor',
	7006: 'Vendor',
	7005: 'Vendor',
	7505: 'Vendor',
	20: 'Lootable',
	3851: 'AbilityTrainer',
	3887: 'AbilityTrainer',
	3888: 'AbilityTrainer',
	3889: 'AbilityTrainer',
	6976: 'AbilityTrainer',
	6975: 'AbilityTrainer'
}

# Required elementary component counts for each component quality when alloying
ALLOYING_ELEMENTARY_COUNTS = {
	Atrea.enums.ITEM_QUALITY_Fantastic: 1,
	Atrea.enums.ITEM_QUALITY_Great: 2,
	Atrea.enums.ITEM_QUALITY_Good: 3,
	Atrea.enums.ITEM_QUALITY_Normal: 5,
	Atrea.enums.ITEM_QUALITY_Poor: 10,
}

# Default height when comparing whether an entity is above/below another
CAS_ENTITY_HEIGHT = 2.5
# Min/max front facing angle
CAS_FRONT_FACING = 0.78539816
# Min/max rear facing angle
CAS_REAR_FACING = 0.78539816
# tan(max_angle) for above/below checks
CAS_ABOVE_TAN = 1.732
# We'll only compare entity height for above/below checks without angle at this distance
CAS_CLOSE_DISTANCE = 5

# Kismet event sets for item handling (Equip/Uneqip/Reload)
ARCHETYPE_ITEM_EVENT_SETS = {
	Atrea.enums.ARCHETYPE_Any: 804,
	Atrea.enums.ARCHETYPE_Soldier: 804,
	Atrea.enums.ARCHETYPE_Commando: 804,
	Atrea.enums.ARCHETYPE_Scientist: 804,
	Atrea.enums.ARCHETYPE_Archeologist: 804,
	Atrea.enums.ARCHETYPE_Asgard: 1455,
	Atrea.enums.ARCHETYPE_Goauld: 804,
	Atrea.enums.ARCHETYPE_Sholva: 804,
	Atrea.enums.ARCHETYPE_Jaffa: 804
}

# Debug log levels
LOG_TRACE = 0
LOG_DEBUG2 = 1
LOG_DEBUG1 = 2
LOG_INFO = 3
LOG_WARN = 4
LOG_ERROR = 5
LOG_CRITICAL = 6

# Maximum distance from an entity when interacting
MAX_INTERACT_DISTANCE = 5

# Maximum character level
# Update the exp leveling table too when changing this!
MAX_LEVEL = 20

# Total experience required to reach each level
# TODO: Replace these values with a proper leveling table
LEVEL_EXP = [
	0,
	# Level 1 - 10
	100, 200, 300, 600, 1000, 1600, 2500, 4000, 6000, 9000,
	# Level 11 - 20
	14000, 18000, 25000, 40000, 60000, 90000, 120000, 180000, 250000, 400000
]


BAG_SIZES = {
	Atrea.enums.INV_Main: 40,
	Atrea.enums.INV_Mission: 100,
	Atrea.enums.INV_Bandolier: 4,
	Atrea.enums.INV_Head: 1,
	Atrea.enums.INV_Face: 1,
	Atrea.enums.INV_Neck: 1,
	Atrea.enums.INV_Chest: 1,
	Atrea.enums.INV_Hands: 1,
	Atrea.enums.INV_Waist: 1,
	Atrea.enums.INV_Back: 1,
	Atrea.enums.INV_Legs: 1,
	Atrea.enums.INV_Feet: 1,
	Atrea.enums.INV_Artifact1: 1,
	Atrea.enums.INV_Artifact2: 1,
	Atrea.enums.INV_Crafting: 100,
	Atrea.enums.INV_Buyback: 12,
	Atrea.enums.INV_Bank: 100,
	Atrea.enums.INV_Auction: 100,
	Atrea.enums.INV_TeamBank: 100,
	Atrea.enums.INV_CommandBank: 100
}
