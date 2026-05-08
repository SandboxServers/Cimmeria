#include <stdafx.hpp>

#include <boost/python/detail/wrap_python.hpp>
#include <entity/pyutil.hpp>
#include <baseapp/entity/ticker.hpp>
#include <common/service.hpp>

Ticker * Ticker::instance_ = nullptr;

void Ticker::initialize(Mercury::Nub * nub)
{
	SGW_ASSERT(!instance_);
	instance_ = new Ticker(nub);
}

Ticker & Ticker::instance()
{
	SGW_ASSERT(instance_);
	return *instance_;
}

Ticker::Ticker(Mercury::Nub * nub)
	: time_(0), nub_(nub)
{
}

Ticker::~Ticker()
{
	nub_->timers().cancelTimer(timerId_);
}

void Ticker::initTicks(uint32_t time, uint32_t tickRate)
{
	time_ = time;
	// TODO: Improve ticking to use ASIO timers instead of Nub (?)
	tickRate_ = tickRate;
	uint64_t now = Service::instance().microTime();
	lastTick_ = now;
	updateTickTimer(now, 1);
}

uint32_t Ticker::time() const
{
	return time_;
}

uint32_t Ticker::tickRate() const
{
	return tickRate_;
}

Ticker::TimerMgr::TimerId Ticker::addEntityTimer(double completeTime, PyObject * callback)
{
	PyUtil_AssertGIL();
	Py_INCREF(callback);
	return timer_.addTimer(boost::bind(&Ticker::expireEntityTimer, this, _4), completeTime, callback);
}

void Ticker::cancelEntityTimer(Ticker::TimerMgr::TimerId timerId)
{
	if (timer_.isValidTimer(timerId))
	{
		PyUtil_AssertGIL();
		PyObject * obj = reinterpret_cast<PyObject *>(timer_.cancelTimer(timerId));
		Py_DECREF(obj);
	}
	else
		WARNC(CATEGORY_SPACE, "Attempted to cancel nonexistent timer %d", timerId);
}

/*
 * Updates the space timer to tick at the next timeslot.
 */
void Ticker::updateTickTimer(uint64_t now, unsigned int ticks)
{
	// Calculate timer drift (difference from expected tick time)
	int drift = (int)(lastTick_ - now);
	// If the drift is more than two ticks (we can't continue ticking normally
	// because the completion time of the next tick has already passed) we need to skip
	// some ticks to be able to continue ticking normally
	if (drift <= -(int)(tickRate_ * 2))
	{
		unsigned int skipTicks = -drift / tickRate_;
		drift += skipTicks * tickRate_;
		WARNC(CATEGORY_SPACE, "Game tick skipped! Ticked at %d, expected %d (drift %d ms, skipped %d ticks)", 
			(int32_t)now, (int32_t)lastTick_, drift, skipTicks);
		lastTick_ += skipTicks * tickRate_;
		time_ += skipTicks;
	}
	timerId_ = nub_->timers().addTimer(boost::bind(&Ticker::tick, this, _1, _2, _3), now + (ticks * tickRate_) + drift);
	lastTick_ += ticks * tickRate_;
}

void Ticker::tick(Mercury::Nub::Timer & mgr, uint64_t timerTime, uint64_t now)
{
	updateTickTimer(now, 1);
	time_++;
	timer_.tick((double)time_ * tickRate_ / 1000.0);
}

void Ticker::expireEntityTimer(void * callback)
{
	PyGilGuard guard;
	PyObject * obj = reinterpret_cast<PyObject *>(callback);
	PyObject * args = PyTuple_New(0);
	PyObject * result = PyObject_Call(obj, args, nullptr);
	Py_XDECREF(result);
	Py_DECREF(args);
	Py_DECREF(obj);
	if (result == nullptr)
	{
		FAULTC(CATEGORY_SPACE, "Error while calling timer handler:");
		PyUtil_ShowErr();
	}
}
