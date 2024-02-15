## Issue-Labeler

This is a proof of concept for a tool that uses an Open Source LLM to label issues on GitHub. It uses a TinyLlama model fine-tuned on existing GitHub issues from 
https://github.com/WasmEdge/WasmEdge. 


The trainning material is obtained by another project https://github.com/jaykchen/training-on-issues

The fine-tuned model is stored at https://huggingface.co/jaykchen/tiny/tree/main.

The model is hosted at http://43.129.206.18:3000/generate to provide predictions for new issues.

The program searches https://github.com/WasmEdge/WasmEdge hourly for new issues, and when it finds one, it uses the same condition/prompts + ChatGPT (exactly like how the model's trainning material was created) to extract key factual information from new issue body and sends it to the model to predict labels. The issue is replicated at the experiment repository https://github.com/jaykchen/issue-labeler, and labeled according to predictions. 

We'll observe the accuracy of the model by comparing the predicted labels with the actual labels of the issue.

Administrators at WasmEdge can use whatever labels they see fit, the labels can change overtime, the model was trained on past issues, so it will pattern match new issues with past issues to predict labels, probably within the known labels pool without creating new ones.


