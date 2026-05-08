#pragma once

#include <mercury/timer.hpp>
#include <mercury/nub.hpp>
#include <boost/python/detail/wrap_python.hpp>

class Ticker
{
public:
	typedef Mercury::TimerMgr<double> TimerMgr;

	static void initialize(class Mercury::Nub * nub);
	static Ticker & instance();

	Ticker(class Mercury::Nub * nub);
	~Ticker();
	
	void initTicks(uint32_t time, uint32_t tickRate);
	uint32_t time() const;
	uint32_t tickRate() const;
	int idleUpdateTicks() const;

	TimerMgr::TimerId addEntityTimer(double completeTime, PyObject * callback);
	void cancelEntityTimer(TimerMgr::TimerId timerId);

private:
	static Ticker * instance_;

	// Current game time on all local spaces (in ticks)
	uint32_t time_;
	// Tickrate of cell (in Nub timer ticks)
	uint32_t tickRate_;
	// When was the last tick (in milliseconds)
	uint64_t lastTick_;
	// ID of tick timer
	Mercury::Nub::Timer::TimerId timerId_;
	// Game ticker (entities can queue events in it)
	TimerMgr timer_;
	class Mercury::Nub * nub_;

	void updateTickTimer(uint64_t now, unsigned int ticks);
	void tick(Mercury::Nub::Timer & mgr, uint64_t timerTime, uint64_t now);
	void expireEntityTimer(void * callback);
};

