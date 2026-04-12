Build a context bundle for the current task using ctxforge.

1. Call `ctxforge_status` to check the current bundle state.
2. Call `ctxforge_recall` to retrieve recent memory notes.
3. Ask the user what they're working on (if not already clear from conversation context).
4. Use the ctxforge skill to build an appropriate context bundle for their task.
5. Report the result: items added, tokens used, percentage of budget.
