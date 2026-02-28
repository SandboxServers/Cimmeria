import Atrea
import Atrea.enums
from common.Logger import warn


class Placeholder(Atrea.Minigame):
	"""
	Placeholder handler for minigames that are not implemented on the client.
	Currently supports Activate, Analyze, Bypass, Converse and Hack.
	"""

	def __init__(self):
		super().__init__()


	###############################################################################################
	#                                SERVER -> CLIENT MESSAGES
	###############################################################################################


	def sendCmd(self, command, args):
		"""
		Sends an extension message to the client

		@param command: Message to send
		@param args: Dictionary of parameters
		"""
		args['_cmd'] = command
		self.send(args)


	def sendFailure(self):
		self.sendCmd('failure', {})


	def sendVictory(self):
		self.sendCmd('victory', {})


	###############################################################################################
	#                            CLIENT -> SERVER MESSAGE HANDLERS
	###############################################################################################


	def onVictory(self):
		self.sendVictory()
		self.victory()
		return True


	###############################################################################################
	#                                  INTERNAL FUNCTIONS
	###############################################################################################


	def started(self):
		pass


	def aborted(self):
		self.started = False
		self.sendFailure()


	def message(self, cmd, args):
		if cmd == 'victory':
			return self.onVictory()
		else:
			warn(Atrea.enums.LC_Minigame, 'Got invalid Placeholder message "%s"!' % cmd)
			return False


