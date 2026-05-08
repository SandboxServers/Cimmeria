import common.Constants
import Atrea.config

class Config:
	# Works around a bug where the radius attribute of client hinted regions doesn't work
	GENERIC_REGION_RADIUS_WORKAROUND = True

	# Is developer mode enabled?
	DEVELOPER_MODE = Atrea.config.developer_mode == "true"

	# Maximum view distance for Kismet sequences
	MAX_KISMET_VIEW_DISTANCE = 300.0

	# Maximum allowed distance from region when checking region enter/leave messages
	GENERIC_REGION_CHECK_THRESHOLD = 1.5

	# Don't allow the same player to join twice
	# Only disable this for development/debugging purposes!
	DISALLOW_DUPLICATE_CHARACTERS = True and not DEVELOPER_MODE

	# The base value of alpha and beta arguments of the beta distribution used when
	# calculating a result code from the QR rating.
	# Lower values result in a flatter histogram and higher base probability for
	# special result codes (miss/glance/critical/double critical)
	QR_ALPHA_BETA = 1.4

	# QR rating multiplier (higher value means that a bigger QR rating difference
	# affects combat results more)
	QR_MULTIPLIER = 2.0

	# QR damage multiplier (damage = baseDamage * qr * QR_DAMAGE_MULTIPLIER)
	QR_DAMAGE_MULTIPLIER = 2.0

	# Value ranges for QR result codes
	# MISS < GLANCING < HIT < CRITICAL_HIT < DOUBLE_CRITICAL_HIT
	# eg. MISS = 0.07 and GLANCING = 0.20 means that values 0.00 .. 0.07 result in
	# RC_MISS and values 0.07 .. 0.20 result in RC_GLANCING
	QR_MISS = 0.07
	QR_GLANCING = 0.20
	QR_CRITICAL_HIT = 0.80
	QR_DOUBLE_CRITICAL_HIT = 0.93

	# Character height when checking for above/below position
	ENTITY_HEIGHT = 6.0

	DEFAULT_LOG_LEVEL = common.Constants.LOG_TRACE

	# Minigame GameId -> GameName mappings
	MINIGAME_NAMES = {
		1: "Activate",
	    2: "Analyze",
	    3: "Bypass",
	    4: "Converse",
	    5: "ConverseBasicHumanoid",
	    6: "CrystalGame",
	    7: "Hack",
	    8: "Livewire",
		9: "GoauldCrystals",
		10: "Alignment"
	}
