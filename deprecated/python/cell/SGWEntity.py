import Atrea
from common.Event import EventParticipant


class SGWEntity(Atrea.CellEntity, EventParticipant):
	def __init__(self):
		Atrea.CellEntity.__init__(self)
		EventParticipant.__init__(self)