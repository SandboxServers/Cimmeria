import Atrea
import Atrea.enums
from xml.sax.saxutils import quoteattr
from common.defs.Resource import Resource

class DialogScreenButton(object):
	__slots__ = ('id', 'type', 'text')

	def __init__(self, row):
		self.id = row['button_id']
		self.type = row['button_type']
		self.text = row['text']

class DialogScreen(object):
	__slots__ = ('id', 'speakerId', 'text', 'buttons', 'index')

	def __init__(self, row):
		self.id = row['screen_id']
		self.speakerId = row['speaker_id']
		self.text = row['text']
		self.index = row['index']
		self.buttons = {}

class Dialog(Resource):
	versioning = True

	def __init__(self, row, defMgr):
		self.id = row['dialog_id']
		self.flags = row['dialog_flags']
		self.kismetEventSetId = row['event_set_id']
		self.uiScreenType = Atrea.enums.__dict__[row['ui_screen_type']]
		self.missionId = row['accepts_mission_id']
		self.screens = {}
		self.screensByIndex = []


	def toXml(self):
		""" Creates a cooked XML resource from the dialog definition """
		xml = '<?xml version="1.0" encoding="UTF-8"?>'
		xml += '<COOKED_DIALOG DialogFlags="'+str(self.flags)+'" DialogID="'+str(self.id)+'" KismetEventSetID="'+str(self.kismetEventSetId or 0)+'" UIScreenType="'+str(self.uiScreenType)+'">'
		for screen in self.screensByIndex:
			xml += '<Screens SpeakerID="'+str(screen.speakerId or 0)+'" ScreenID="'+str(screen.id)+'" Text='+quoteattr(screen.text)+'>'
			for button in screen.buttons.values():
				xml += '<Buttons ButtonID="'+str(button.id)+'" ButtonType="'+str(button.type)+'" Text='+quoteattr(button.text)+' />'
			xml += '</Screens>'
		xml += '</COOKED_DIALOG>'
		return xml

	@staticmethod
	def loadAll(defs, defMgr):
		dialogs = Atrea.dbQuery('select * from resources.dialogs')
		for dialog in dialogs:
			defs[dialog['dialog_id']] = Dialog(dialog, defMgr)

		screenDialogs = {}
		screens = Atrea.dbQuery('select * from resources.dialog_screens order by index')
		for s in screens:
			screen = DialogScreen(s)
			screenDialogs[screen.id] = s['dialog_id']
			dlg = defs[s['dialog_id']]
			dlg.screens[screen.id] = screen
			assert len(dlg.screensByIndex) == screen.index
			dlg.screensByIndex.append(screen)

		buttons = Atrea.dbQuery('select * from resources.dialog_screen_buttons')
		for b in buttons:
			button = DialogScreenButton(b)
			defs[screenDialogs[b['screen_id']]].screens[b['screen_id']].buttons[button.id] = button
