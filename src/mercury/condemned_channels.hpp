#pragma once

#include <mercury/nub.hpp>
#include <mercury/channel.hpp>

namespace Mercury {

class CondemnedChannels
{
public:
	CondemnedChannels(Nub & nub);
	~CondemnedChannels();

	void addChannel(BaseChannel::Ptr channel);

private:
	// How often (in milliseconds) do we poll channels to see if any can be deleted
	static const unsigned int PollInterval = 5000;
	// How long will we wait for activity on the channel until we consider it dead
	static const unsigned int InactiveChannelTimeout = 15000;

	Nub & nub_;
	Nub::Timer::TimerId timerId_;
	std::vector<BaseChannel::Ptr> channels_;

	void pollChannels(uint64_t now);
};

}