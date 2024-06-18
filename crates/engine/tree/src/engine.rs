//! An engine API handler for the chain.

use crate::{
    chain::{ChainHandler, FromOrchestrator, HandlerEvent, OrchestratorState},
    download::{BlockDownloader, DownloadAction, DownloadOutcome},
    tree::EngineApiTreeHandler,
};
use futures::{stream::Fuse, Stream, StreamExt};
use reth_beacon_consensus::BeaconEngineMessage;
use reth_primitives::{SealedBlockWithSenders, B256};
use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
};

/// Advances the chain based on incoming requests.
///
/// This is a general purpose request handler with network access.
/// This type listens for incoming messages and processes them via the configured request handler.
///
/// ## Overview
///
/// This type is an orchestrator for incoming messages and responsible for delegating requests
/// received from the CL to the handler.
///
/// It is responsible for handling the following:
/// - Downloading blocks on demand from the network if requested by the [`EngineApiRequestHandler`].
///
/// The core logic is part of the [`EngineRequestHandler`], which is responsible for processing the
/// incoming requests.
pub struct EngineHandler<T, D>
where
    T: EngineRequestHandler,
{
    /// Processes requests.
    ///
    /// This type is responsible for processing incoming requests.
    handler: T,
    /// Receiver for incoming requests that need to be processed.
    // TODO maybe use generic?
    incoming_requests: Fuse<Pin<Box<dyn Stream<Item = T::Request> + Send + Sync + 'static>>>,
    /// A downloader to download blocks on demand.
    downloader: D,
}

impl<T, D> ChainHandler for EngineHandler<T, D>
where
    T: EngineRequestHandler,
    D: BlockDownloader,
{
    type Event = T::Event;

    fn on_event(&mut self, event: FromOrchestrator) {
        // delegate event to the handler
        self.handler.on_event(event.into());
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<HandlerEvent<Self::Event>> {
        loop {
            // drain the handler first
            loop {
                match self.handler.poll(cx) {
                    Poll::Ready(ev) => {
                        match ev {
                            RequestHandlerEvent::Idle => break,
                            RequestHandlerEvent::HandlerEvent(ev) => {
                                match ev {
                                    HandlerEvent::Pipeline(target) => {
                                        // bubble up pipeline request
                                        // TODO: clear downloads in progress
                                        return Poll::Ready(HandlerEvent::Pipeline(target))
                                    }
                                    HandlerEvent::Event(ev) => {
                                        // bubble up the event
                                        return Poll::Ready(HandlerEvent::Event(ev));
                                    }
                                }
                            }
                            RequestHandlerEvent::Download(req) => {
                                // delegate download request to the downloader
                                self.downloader.on_action(DownloadAction::Download(req));
                            }
                        }
                    }
                    Poll::Pending => break,
                }
            }

            // pop the next incoming request
            if let Poll::Ready(Some(req)) = self.incoming_requests.poll_next_unpin(cx) {
                // and delegate the request to the handler
                self.handler.on_event(FromEngine::Request(req));
                // skip downloading in this iteration to allow the handler to process the request
                continue
            }

            // advance the downloader
            if let Poll::Ready(DownloadOutcome::Blocks(blocks)) = self.downloader.poll(cx) {
                // delegate the downloaded blocks to the handler
                self.handler.on_event(FromEngine::DownloadedBlocks(blocks));
                continue
            }

            return Poll::Pending
        }
    }
}

/// A type that processes incoming requests (e.g. requests from the consensus layer, engine API)
pub trait EngineRequestHandler: Send + Sync {
    /// Even type this handler can emit
    type Event: Send;
    /// The request type this handler can process.
    type Request;

    /// Informs the handler about an event from the [`EngineHandler`].
    fn on_event(&mut self, event: FromEngine<Self::Request>);

    /// Advances the handler.
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<RequestHandlerEvent<Self::Event>>;
}

/// An [`EngineRequestHandler`] that processes engine API requests.
///
/// This type is responsible for advancing the chain during live sync (following the tip of the
/// chain).
///
/// It advances the chain based on received engine API requests:
///
/// - `on_new_payload`: Executes the payload and inserts it into the tree. These are allowed to be
///   processed concurrently.
/// - `on_forkchoice_updated`: Updates the fork choice based on the new head. These require write
///   access to the database and are skipped if the handler can't acquire exclusive access to the
///   database.
///
/// The [`EngineApiTreeHandler`] is used to execute the incoming payloads, storing them in a tree
/// structure and committing new chains to the database on fork choice updates.
///
/// In case required blocks are missing, the handler will request them from the network, by emitting
/// a download request upstream.
#[derive(Debug)]
pub struct EngineApiRequestHandler<T>
where
    T: EngineApiTreeHandler,
{
    /// The state of the top level orchestrator.
    orchestrator_state: OrchestratorState,
    /// Needs to keep track of requested state changes by the orchestrator.
    state: (),
    /// Currently in progress payload requests.
    pending_payloads: Vec<()>,
    /// Currently in progress fork choice updates.
    pending_fcu: Option<()>,
    /// Next FCU to process
    next_fcu: VecDeque<()>,
    /// Manages the tree
    tree_handler: T,
    /// Events to yield.
    buffered_events: VecDeque<()>,
}

impl<T> EngineRequestHandler for EngineApiRequestHandler<T>
where
    T: EngineApiTreeHandler,
{
    // TODO: add events for reporting
    type Event = ();
    type Request = BeaconEngineMessage<T::Engine>;

    fn on_event(&mut self, event: FromEngine<Self::Request>) {
        // TODO check if we're currently syncing, or mutable access is currently held by the
        // orchestrator then we respond with SYNCING or delay forkchoice updates.  otherwise
        // we tell the tree to handle the requests, but we likely still need to tell the handler
        // about the stuff while write access is unavailable.

        // TODO: should this type spawn the jobs and have access to tree internals or should this be
        // entirely handled by the tree handler? basically

        /*
           let (tx, req) = event;
           let handler = self.tree_handler.clone();
           spawn(move {
               let resp = handler.on_new_payload(req);
               tx.send(resp).unwrap();
           });
        */
        // here the handler must contain shareable state internally

        // or

        /*
         let fut = handler.handle(event);
         self.pending.push(fut);
        */

        // the latter would give the handler more control over the execution of the requests, with
        // this model the logic of this type and the tree handler is very similar

        todo!()
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<RequestHandlerEvent<Self::Event>> {
        // advance tree tasks, trigger
        todo!()
    }
}

#[derive(Debug)]
pub enum FromEngine<Req> {
    Event(FromOrchestrator),
    Request(Req),
    DownloadedBlocks(Vec<SealedBlockWithSenders>),
}

impl<Req> From<FromOrchestrator> for FromEngine<Req> {
    fn from(event: FromOrchestrator) -> Self {
        Self::Event(event)
    }
}

#[derive(Debug)]
pub enum RequestHandlerEvent<T> {
    Idle,
    HandlerEvent(HandlerEvent<T>),
    Download(DownloadRequest),
}

/// A request to download blocks from the network.
#[derive(Debug)]
pub enum DownloadRequest {
    Blocks(Vec<B256>),
    Range(B256, usize),
}
