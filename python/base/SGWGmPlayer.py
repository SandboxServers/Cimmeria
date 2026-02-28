import Atrea
import time
from base import Chat
from base.SGWPlayer import SGWPlayer
from common import debug

class SGWGmPlayer(SGWPlayer):
	account = None

	def __init__(self):
		super().__init__()


	def loadConstants(self):
		"""
		CellApp RPC (repurposed)
		Reloads all Python scripts on the server
		"""
		self.debug("Reloading Python scripts on BaseApp ...")
		start = time.clock()
		channelMgr = Chat.ChannelManager
		debug.reloadModules()
		debug.reloadValue(channelMgr, {'reloaded': 0})
		Chat.ChannelManager = channelMgr
		self.debug("Reloaded BaseApp scripts in %f sec" % (time.clock() - start))

