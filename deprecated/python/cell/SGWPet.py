import Atrea
from cell.SGWMob import SGWMob

class SGWPet(SGWMob):
	abilityList = []
	stanceList = []
	
	def __init__(self):
		super().__init__()
		self.abilityList = []
		self.stanceList = []

	
	def createOnClient(self, mailbox):
		mailbox.onPetAbilityList(self.abilityList)
		mailbox.onPetStanceList(self.stanceList)
		#mailbox.onPetStanceUpdate(self.stanceList)
