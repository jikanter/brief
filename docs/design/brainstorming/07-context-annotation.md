# Context Annotation

Something that I find interesting about how some application harnesses interact is that they often use MDX to 
annotate existing sub-sections of text, allowing for prompt injection as a fail-safe if they believe it will 
improve on the results for the user.

What my true wondering is if MDX is a good general-purpose fit for brief, i.e. if "brief:generated" is just the beginning 
of my own exploration into MDX-driven annotation. But what if it could also be the reverse? Could we use the existing 
annotations of systems to analyze how brief generates? 

In other words, could we use MDX as an intermediate representation of a target format for brief? Rather than seeing 
MDX as presentational only, is it possible that there is something semantic about how Anthropic and other providers
use MDX?