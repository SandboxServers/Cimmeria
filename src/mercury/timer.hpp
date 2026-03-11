#pragma once

#include <mercury/bundle.hpp>

namespace Mercury {

template <typename _TIME>
class TimerMgr
{
public:
	typedef std::function<void (TimerMgr &, _TIME, _TIME, void *)> TimerHandler;
	typedef uint32_t TimerId;

	TimerMgr()
	{
	}

	TimerId addTimer(TimerHandler handler, _TIME time, void * userData = nullptr)
	{
		TimerQueueEntry ent;
		ent.time = time;

		if (freeHandlers_.empty())
		{
			ent.handlerId = (TimerId)handlers_.size();
			handlers_.push_back(std::make_pair(handler, userData));
		}
		else
		{
			ent.handlerId = freeHandlers_.top();
			freeHandlers_.pop();
			SGW_ASSERT(!handlers_[ent.handlerId].first);
			handlers_[ent.handlerId].first = handler;
			handlers_[ent.handlerId].second = userData;
		}

		timers_.push(ent);
		return ent.handlerId;
	}

	void * cancelTimer(TimerId timer)
	{
		SGW_ASSERT(timer < handlers_.size() && handlers_[timer].first);
		handlers_[timer].first = nullptr;
		return handlers_[timer].second;
	}

	bool isValidTimer(TimerId timer)
	{
		return timer < handlers_.size() && handlers_[timer].first;
	}

	void tick(_TIME time)
	{
		while (!timers_.empty() && timers_.top().time <= time)
		{
			TimerQueueEntry timer = timers_.top();
			TimerHandler callback = handlers_[timer.handlerId].first;
			handlers_[timer.handlerId].first = nullptr;
			freeHandlers_.push(timer.handlerId);
			timers_.pop();
			if (callback)
				callback(std::ref(*this), timer.time, time, handlers_[timer.handlerId].second);
		}
	}
	
private:
	struct TimerQueueEntry
	{
		TimerId handlerId;
		_TIME time;

		inline bool operator > (TimerQueueEntry const & t) const
		{
			return time > t.time;
		}
	};

	std::priority_queue<TimerQueueEntry, std::vector<TimerQueueEntry>, std::greater<TimerQueueEntry> > timers_;
	std::vector<std::pair<TimerHandler, void *> > handlers_;
	std::stack<TimerId> freeHandlers_;
};

}
