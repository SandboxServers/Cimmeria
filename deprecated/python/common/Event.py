from common.Logger import warn, trace, error, exc
import Atrea.enums


class EventParticipant(object):
	subscriptions = None
	subscriptionsByEvent = None
	nextSubscriptionId = 1


	def __init__(self):
		super().__init__()
		self.subscriptions = {}
		self.subscriptionsByEvent = {}


	def subscribe(self, event, callback, once = False):
		"""
		Adds a subscription to the specified event.
		When the event is triggered via fire() or first(), the callback function is called with
		parameters supplied by the event source.
		@param event: Name of event to subscribe to
		@param callback: Callback function
		@param once: Fire only once (auto unregister)
		@return: Subscription ID (used to unsubscribe)
		"""
		id = self.nextSubscriptionId
		sub = {
			'event': event,
			'callback': callback,
			'id': id,
			'once': once
		}
		self.subscriptions[id] = sub
		if event not in self.subscriptionsByEvent:
			self.subscriptionsByEvent[event] = {id: sub}
		else:
			self.subscriptionsByEvent[event][id] = sub
		self.nextSubscriptionId += 1
		return id


	def unsubscribe(self, id):
		"""
		Removes a subscription by its id.
		@param id: Subscription id
		@return: True if successful, False if the id is invalid
		"""
		if id not in self.subscriptions:
			warn(Atrea.enums.LC_Event, 'EventParticipant::unsubscribe(): Illegal subscription id %d' % id)
			return False

		sub = self.subscriptions[id]
		del self.subscriptions[id]
		del self.subscriptionsByEvent[sub['event']][id]
		return True


	def fire(self, event, **args):
		"""
		Fires an event
		Calls all event handlers registered to the specified event.
		@param event: Event name to trigger
		@param args: Optional event parameters
		"""
		if event in self.subscriptionsByEvent:
			trace(Atrea.enums.LC_Event, "Event '%s' fired with %d subscribers" % (event, len(self.subscriptionsByEvent[event])))
			try:
				unsub = []
				for sub in list(self.subscriptionsByEvent[event].values()):
					if sub['once']:
						unsub.append(sub['id'])
					sub['callback'](args)
				for id in unsub:
					self.unsubscribe(id)
			except Exception:
				error(Atrea.enums.LC_Event, "Exception while triggering event '%s'" % event)
				exc(Atrea.enums.LC_Event)
		else:
			trace(Atrea.enums.LC_Event, "Event '%s' fired with no subscribers" % event)


	def first(self, event, **args):
		"""
		Fires an event
		Calls event handlers registered to the specified event until
		one of them returns a value other than None.
		@param event: Event name to trigger
		@param args: Optional event parameters
		"""
		ret = None
		if event in self.subscriptionsByEvent:
			trace(Atrea.enums.LC_Event, "Event '%s' firsted with %d subscribers" % (event, len(self.subscriptionsByEvent[event])))
			try:
				unsub = []
				for sub in list(self.subscriptionsByEvent[event].values()):
					if sub['once']:
						unsub.append(sub['id'])
					ret = sub['callback'](args)
					if ret is not None:
						break
				for id in unsub:
					self.unsubscribe(id)
			except Exception:
				error(Atrea.enums.LC_Event, "Exception while triggering event '%s'" % event)
				exc(Atrea.enums.LC_Event)
		else:
			trace(Atrea.enums.LC_Event, "Event '%s' firsted with no subscribers" % event)
		return ret

