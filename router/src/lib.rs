use smallvec::SmallVec;
use std::fmt::Debug;

pub trait Endpoint: Debug {
    /// Strict matching will only match on perfect matches
    /// Setting this to false will allow paths that only match at the start to still match if no more precise route is available
    fn use_strict_path_matching(&self) -> bool {
        true
    }

    //TODO Maybe change `path_overload` to provide a string(s) instead of `&[Vec<u8>]`
    // The reason this provides a `&[Vec<u8>]` instead of either a `&[String]` or `String` is to minimize overhead when calling. However, it can be argued that any endpoint that wants this will want it as string form.
    fn process(&self, path_overload: Option<&[SmallVec<[u8; 5]>]>);
}

/// A router path is a string path (e.g. "some/router/to/somewhere") that is split at '/' and each part is represented as a series of bytes.
///
#[derive(Default, Debug)]
pub struct RouterPath {
    parts: SmallVec<[SmallVec<[u8; 5]>; 10]>,
}

impl From<SmallVec<[SmallVec<[u8; 5]>; 10]>> for RouterPath {
    fn from(data: SmallVec<[SmallVec<[u8; 5]>; 10]>) -> Self {
        Self { parts: data }
    }
}

impl From<&str> for RouterPath {
    /// This splits the provided `&str` at '/' and converts it to a router path
    fn from(path: &str) -> Self {
        let parts = path.split('/');

        let parts: SmallVec<[SmallVec<[u8; 5]>; 10]> =
            parts.map(|part| SmallVec::from(part.as_bytes())).collect();

        parts.into()
    }
}

/// Router
///
/// This stores added paths in `matches`, which is a byte representation of the bytes
/// `0` bytes indicate a path boundary.
/// `router` contains the path's matching routers in the same order
///
/// This is intended to be a cache friendly router for low amounts of low-length path
///
//TODO testing needs to be done to see if this is actually faster than a string array or hashmap alternative
//TODO Since a byte comparison is made, it should be an easy simd candidate. Either it needs to verify that the compiler will generate simd for this or it should be implemented manually
//TODO Would it be simpler to use chars instead of bytes here? Does it matter, is it faster?
#[derive(Default, Debug)]
pub struct Router {
    endpoint: Option<Box<Endpoint>>,
    matches: SmallVec<[u8; 20]>,
    routers: SmallVec<[Box<Router>; 5]>,
}

impl Router {
    /// Attempt to match a path part to a router
    /// Returns `None` if no math is found
    /// Returns `Some(index)` if a match was found. This index can be used to access the match using `Router.matches`
    fn find_path_part_match(&self, path: &[u8]) -> Option<usize> {
        let mut current_match_index = 0;
        let mut current_path_index = 0;

        let mut match_result = None;

        let mut current_path = 0;

        //TODO This loop is a bit messy and does not convey meaning too well, could be re-written to be better
        // Be carefull of the <= for current_path_index, it's intentionally off by 1. This probably needs to be made better
        while current_path_index <= path.len() && current_match_index < self.matches.len() {
            let byte_to_match = self.matches[current_match_index];

            if byte_to_match == 0 {
                // If we are at end of path boundary, check if path contains anything else to match on
                // If not, match is a success
                if current_path_index >= path.len() {
                    match_result = Some(current_path);
                    break;
                }

                // On '0' byte, this means we passed a path boundary and we need to restart matching on the next path
                // We also need to reset the input path since the matching is starting from the start again
                current_match_index += 1;
                current_path_index = 0;

                current_path += 1;
                continue;
            }

            // In the while condition, we allow `current_path_index` to go 1 past it's limit to see if theirs a 0 as a matching byte.
            // This means that if their was no zero, we need to exit the loop right now to prevent out of bounds access
            if current_path_index > path.len() - 1 {
                break;
            }

            // If their is a path mismatch, we advance past next path boundary and reset path match index
            if byte_to_match != path[current_path_index] {
                while current_match_index < self.matches.len() {
                    if self.matches[current_match_index] == 0 {
                        current_match_index += 1;
                        current_path += 1;
                        break;
                    }
                    current_match_index += 1;
                }
                current_path_index = 0;

                continue;
            }

            // We get here if their is a match, in that case, just continue matching
            current_match_index += 1;
            current_path_index += 1;
        }

        match_result
    }

    /// Add a path to the router that maps to a specified endpoint
    pub fn add_path(&mut self, path: impl Into<RouterPath>, endpoint: impl Endpoint + 'static) {
        let mut current_router = self;
        let path = path.into();

        for part in &path.parts {
            if let Some(match_index) = current_router.find_path_part_match(part) {
                let next_router = &mut current_router.routers[match_index];
                current_router = next_router;
            } else {
                current_router.matches.extend_from_slice(&part);
                current_router.matches.push(0);

                let new_router = Box::new(Router::default());
                current_router.routers.push(new_router);

                let new_router_index = current_router.routers.len() - 1;
                current_router = &mut current_router.routers[new_router_index];
            }
        }

        current_router.endpoint = Some(Box::new(endpoint));
    }

    /// Attempt to route a query to an endpoint
    pub fn route(&self, path: impl Into<RouterPath>) {
        let path = path.into();

        let mut current_router = self;

        // This is needed to give the path overload to the endpoint if needed
        let mut failed_to_match = false;
        let mut last_path_index = None;

        for (path_index, part) in path.parts.iter().enumerate() {
            if let Some(match_index) = current_router.find_path_part_match(part) {
                last_path_index = Some(path_index);
                current_router = &current_router.routers[match_index];
            } else {
                failed_to_match = true;
                break;
            }
        }

        if let Some(endpoint) = &current_router.endpoint {
            if endpoint.use_strict_path_matching() {
                if !failed_to_match {
                    endpoint.process(None);
                }
            } else if let Some(last_path_index) = last_path_index {
                endpoint.process(Some(&path.parts[last_path_index..]));
            } else {
                endpoint.process(None);
            }
        }
    }
}